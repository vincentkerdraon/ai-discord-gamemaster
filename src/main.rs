// stolen from https://github.com/serenity-rs/songbird/blob/current/examples/serenity/voice/src/main.rs
// (I had my own code, but it would fail when joining and I never figured why)
// This is working out of the box
// But with deprecation warnings
#![allow(deprecated)]

use openai_api::{run_completion, text_to_speech, AssistantRequest};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::time::Duration;
use std::{env, error::Error};
use tracing::{debug, info, warn};

use songbird::id::GuildId;
// This trait adds the `register_songbird` and `register_songbird_with` methods
// to the client builder below, making it easy to install this voice client.
// The voice client can be retrieved in any command using `songbird::get(ctx).await`.
use songbird::SerenityInit;

// Event related imports to detect track creation failures.
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};

// To turn user URLs into playable audio, we'll use yt-dlp.
use songbird::input::{Input, YoutubeDl};

// YtDl requests need an HTTP client to operate -- we'll create and store our own.
use reqwest::Client as HttpClient;

// Import the `Context` to handle commands.
use serenity::client::Context;

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    framework::{
        standard::{
            macros::{command, group},
            Args, CommandResult, Configuration,
        },
        StandardFramework,
    },
    model::{channel::Message, gateway::Ready, prelude::ReactionType},
    prelude::{GatewayIntents, TypeMapKey},
    Result as SerenityResult,
};

struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(join, leave, play, stop, ping, help, report)]
struct General;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("AI_DISCORD_GM_DISCORD_TOKEN")
        .expect("Expected token AI_DISCORD_GM_DISCORD_TOKEN in the environment");

    let framework = StandardFramework::new().group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix("!")); //FIXME also prefix("/") ?

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        // We insert our own HTTP client here to make use of in
        // `!play`. If we wanted, we could supply cookies and auth
        // details ahead of time.
        //
        // Generally, we don't want to make a new Client for every request!
        .type_map_insert::<HttpKey>(HttpClient::new())
        .await
        .expect("Err creating client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| warn!("Client ended: {:?}", why));
    });

    let _signal_err = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C, shutting down.");
}

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

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    check_msg(&msg.reply(&ctx.http, "Pong!").await);
    Ok(())
}

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
        add_reaction(ctx, &msg, emoji(EMOJI_WAIT)).await?;
    } else {
        check_msg(
            &msg.reply(&ctx.http, "Not in a voice channel to pause")
                .await,
        );
    }

    Ok(())
}

//Custom command to display other commands
#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    //FIXME can I get the current version from Cargo.toml ?
    let help_text = "Available commands:\n\
        - !help: Displays this help message.\n\
        - !ping: Responds with 'Pong!'\n\
        - !join: Joins the voice channel you are currently in.\n\
        - !leave: Leaves the current voice channel.\n\
        - !play <url/search term>: Plays an audio track from a URL or searches YouTube.\n\
        - !stop: stops the current audio.\n\
        - !report <text>: Sends the provided text to the game master assistant and displays the answer. Use reaction to read the response aloud.";

    check_msg(&msg.reply(&ctx.http, help_text).await);
    Ok(())
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: &SerenityResult<Message>) {
    if let Err(why) = result {
        warn!("Error sending message: {:?}", why);
    }
}

#[command]
#[only_in(guilds)]
async fn report(ctx: &Context, msg_user: &Message) -> CommandResult {
    let prompt = msg_user.content[8..].to_string();

    match handle_report(ctx, msg_user, prompt).await {
        Ok(_) => Ok(()),
        Err(e) => {
            check_msg(&msg_user.reply(&ctx.http, format!("Error: {}", e)).await);
            Err(e.into()) // Or handle the error differently if needed
        }
    }
}

const EMOJI_SOUND: &str = "🔊";
const EMOJI_WAIT: &str = "⏳";
const EMOJI_DONE: &str = "✅";
fn emoji(e: &str) -> serenity::all::ReactionType {
    return ReactionType::Unicode(e.to_string());
}

fn pre_prompt(user: &serenity::model::user::User) -> &str {
    match user.id.get() {
        607653619122307123 => "Comm dit:",
        374989552646881281 => "Explo dit:",
        518896639608619022 => "Secu dit:",
        _ => {
            warn!("Unknown user id={}", user.id);
            return "";
        }
    }
}

async fn handle_report(
    ctx: &Context,
    msg_user: &Message,
    mut prompt: String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    prompt = format!("{}{}", pre_prompt(&msg_user.author), prompt);
    let prompt2 = prompt.clone();
    add_reaction(ctx, &msg_user, emoji(EMOJI_WAIT)).await?;

    let assistant_request = AssistantRequest { prompt };
    let text_generated = run_completion(assistant_request).await?;

    let msg_generated = msg_user.reply(&ctx.http, &text_generated).await?;
    debug!("delete_reaction msg_user EMOJI_WAIT",);
    delete_reaction(ctx, msg_user, emoji(EMOJI_WAIT)).await?;
    debug!("add_reaction msg_user EMOJI_DONE",);
    add_reaction(ctx, &msg_user, emoji(EMOJI_DONE).clone()).await?;
    debug!("add_reaction msg_generated EMOJI_SOUND",);
    add_reaction(ctx, &msg_generated, emoji(EMOJI_SOUND)).await?;

    let file_path: String = format!("{}{}", ASSETS_DIR, generate_file_hash(&text_generated));
    let text_path = format!("{}.txt", file_path);

    let text_content: String = format!("{}\n---\n{}", prompt2, text_generated);
    std::fs::write(text_path, text_content).unwrap();

    let guild_id = msg_user.guild_id.unwrap();

    //Wait, else it will detect it's own adding of the emoji.
    //A better way would be to filter out itself.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    react_and_handle_response(
        ctx,
        guild_id.into(),
        msg_generated,
        text_generated,
        file_path.as_str(),
    )
    .await?;
    Ok(())
}

async fn react_and_handle_response(
    ctx: &Context,
    guild_id: GuildId,
    msg: Message,
    text_generated: String,
    hash_file_name: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let msg_generated2 = msg.clone();

    loop {
        let collector = msg
            .await_reaction(ctx)
            .timeout(Duration::from_secs(600))
            .await;

        if collector.is_none() {
            return Ok(());
        }
        let reaction = collector.unwrap();
        if reaction.emoji != emoji(EMOJI_SOUND) {
            continue;
        }
        debug!("delete_reaction msg EMOJI_SOUND",);
        delete_reaction(ctx, &msg, emoji(EMOJI_SOUND)).await?;
        debug!("add_reaction msg EMOJI_WAIT",);
        add_reaction(ctx, &msg, emoji(EMOJI_WAIT)).await?;
        handle_reaction(
            ctx,
            guild_id,
            &msg_generated2,
            &text_generated,
            hash_file_name,
            reaction,
        )
        .await?;

        debug!("delete_reaction msg EMOJI_WAIT",);
        delete_reaction(ctx, &msg, emoji(EMOJI_WAIT)).await?;
        debug!("add_reaction msg EMOJI_DONE",);
        add_reaction(ctx, &msg, emoji(EMOJI_DONE)).await?;

        return Ok(());
    }
}

async fn add_reaction(
    ctx: &Context,
    msg: &Message,
    reaction: ReactionType,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    msg.react(&ctx.http, reaction).await?;
    Ok(())
}

async fn delete_reaction(
    ctx: &Context,
    msg: &Message,
    reaction: ReactionType,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    msg.delete_reaction(&ctx.http, None, reaction).await?;
    Ok(())
}

const ASSETS_DIR: &str = "assets/";

async fn handle_reaction(
    ctx: &Context,
    guild_id: GuildId,
    msg: &Message,
    text_generated: &str,
    hash_file_name: &str,
    reaction: serenity::model::channel::Reaction,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let ReactionType::Unicode(emoji) = reaction.emoji {
        if emoji == EMOJI_SOUND {
            let ctx = ctx.clone();
            let audio_path = format!("{}.opus", hash_file_name);

            match text_to_speech(text_generated, &audio_path).await {
                Ok(_) => {
                    if let Err(why) = read_local_audio(&ctx, guild_id, msg, &audio_path).await {
                        check_msg(
                            &msg.reply(&ctx.http, &format!("Audio playback failed: {:?}", why))
                                .await,
                        );
                        return Err(why);
                    }
                }
                Err(why) => {
                    check_msg(
                        &msg.reply(&ctx.http, &format!("Text-to-speech failed: {:?}", why))
                            .await,
                    );
                    return Err(why);
                }
            }
        }
    }
    Ok(())
}

pub async fn read_local_audio(
    ctx: &Context,
    guild_id: GuildId,
    msg: &Message,
    audio_path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    debug!("read_local_audio {}", audio_path);

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        handler.stop();

        let mut file = std::fs::File::open(audio_path)
            .map_err(|e| format!("Failed to open file: {} - {}", audio_path, e))?;
        let mut audio_data = Vec::new();
        file.read_to_end(&mut audio_data)
            .map_err(|e| format!("Failed to read file data: {}", e))?;
        let src = Input::from(audio_data);

        handler.play_input(src.into());
        Ok(())
    } else {
        let error_message = "Not in a voice channel to play in";
        check_msg(&msg.reply(&ctx.http, error_message).await);
        Err(error_message.into())
    }
}

fn generate_file_hash(text_generated: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text_generated.as_bytes());
    let result = hasher.finalize();
    let hash_hex = hex::encode(result);
    hash_hex[..8].to_string()
}
