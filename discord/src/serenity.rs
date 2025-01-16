// stolen from https://github.com/serenity-rs/songbird/blob/current/examples/serenity/voice/src/main.rs
// (I had my own code, but it would fail when joining and I never figured why)
// This template is working out of the box
// But with deprecation warnings
#![allow(deprecated)]

use serenity::all::standard::Configuration;
use serenity::all::{GatewayIntents, StandardFramework};
use serenity::Client;
use std::sync::Arc;
use text_completion::RequestHandler;
use tracing::{info, warn};

// This trait adds the `register_songbird` and `register_songbird_with` methods
// to the client builder below, making it easy to install this voice client.
// The voice client can be retrieved in any command using `songbird::get(ctx).await`.
use songbird::SerenityInit;

// Event related imports to detect track creation failures.
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};

// To turn user URLs into playable audio, we'll use yt-dlp.
use songbird::input::YoutubeDl;

// Import the `Context` to handle commands.
use serenity::client::Context;

use serenity::{
    async_trait,
    client::EventHandler,
    framework::standard::{
        macros::{command, group},
        Args, CommandResult,
    },
    model::{channel::Message, gateway::Ready},
};

use crate::reaction::{add_reaction, emoji, EMOJI_WAIT};
use crate::{check_msg, serenity_report, DiscordHandler, HttpKey, PREFIX};

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg_user: Message) {
        //instead of using automatic routing with #command,
        //we move this code here to have directly the handler
        // else we would need all the serenity context reading

        if msg_user.author.bot {
            return;
        }
        let prefix = PREFIX.to_string() + "report ";
        if !msg_user.content.starts_with(&prefix) {
            return;
        }
        let prompt = msg_user.content[prefix.len()..].to_string();

        match serenity_report::handle_report(&ctx, self, &msg_user, &prompt).await {
            Ok(_) => return,
            Err(e) => {
                check_msg(&msg_user.reply(&ctx.http, format!("Error: {}", e)).await);
                return;
            }
        }
    }
}

pub async fn init(token: &str, request_handler: Arc<dyn RequestHandler + Send + Sync + 'static>) {
    let framework = StandardFramework::new().group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix(PREFIX));

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let h: DiscordHandler = DiscordHandler { request_handler };

    let mut client = Client::builder(token, intents)
        .event_handler(h)
        .framework(framework)
        .register_songbird()
        // We insert our own HTTP client here to make use of in
        // `!play`. If we wanted, we could supply cookies and auth
        // details ahead of time.
        //
        // Generally, we don't want to make a new Client for every request!
        .type_map_insert::<HttpKey>(reqwest::Client::new())
        .await
        .expect("Err creating client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| warn!("Client ended: {:?}", why));
    });
}

#[group]
#[commands(join, leave, play, stop, help, report)]
struct General;

//Custom command to display other commands
#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    //FIXME can I get the current version from Cargo.toml ?
    let help_text = "Available commands:\n\
        - !help: Displays this help message.\n\
        - !join: Joins the voice channel you are currently in.\n\
        - !leave: Leaves the current voice channel.\n\
        - !play <url/search term>: Plays an audio track from a URL or searches YouTube.\n\
        - !stop: Stops the current audio.\n\
        - !report <text>: Sends the provided text to the game master assistant and displays the answer. Use reaction to read the response aloud.";

    check_msg(&msg.reply(&ctx.http, help_text).await);
    Ok(())
}

//Custom command to chat
#[command]
#[only_in(guilds)]
async fn report(_: &Context, _: &Message) -> CommandResult {
    //This is done in the trait and not using #[command], because we want to use DiscordHandler without going throught the serenity context
    Ok(())
}

// stolen from example
#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let (guild_id, channel_id) = {
        let guild = msg.guild(&ctx.cache).unwrap();
        let channel_id = guild
            .voice_states
            .get(&msg.author.id)
            .and_then(|voice_state| voice_state.channel_id);

        (guild.id, channel_id)
    };

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(&msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Ok(handler_lock) = manager.join(guild_id, connect_to).await {
        // Attach an event handler to see notifications of all track errors.
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
    }

    Ok(())
}

struct TrackErrorNotifier;

// stolen from example
#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                warn!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }
        None
    }
}

// stolen from example
#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            check_msg(&msg.reply(&ctx.http, format!("Failed: {:?}", e)).await);
        }

        check_msg(&msg.reply(&ctx.http, "Left voice channel").await);
    } else {
        check_msg(&msg.reply(ctx, "Not in a voice channel").await);
    }

    Ok(())
}

// stolen from example
#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            check_msg(
                &msg.reply(&ctx.http, "Must provide a URL to a video or audio")
                    .await,
            );

            return Ok(());
        }
    };

    let do_search = !url.starts_with("http");

    let guild_id = msg.guild_id.unwrap();

    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        handler.stop();
        let src = if do_search {
            YoutubeDl::new_search(http_client, url)
        } else {
            YoutubeDl::new(http_client, url)
        };
        let _ = handler.play_input(src.clone().into());
    } else {
        check_msg(
            &msg.reply(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

// stolen from example
#[command]
#[only_in(guilds)]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        handler.stop();
        add_reaction(ctx, msg, emoji(EMOJI_WAIT)).await?;
    } else {
        check_msg(
            &msg.reply(&ctx.http, "Not in a voice channel to pause")
                .await,
        );
    }

    Ok(())
}
