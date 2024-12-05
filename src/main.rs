// stolen from https://github.com/serenity-rs/songbird/blob/current/examples/serenity/voice/src/main.rs
// (I had my own code, but it would fail when joining and I never figured why)
// This is working out of the box
// But with deprecation warnings
#![allow(deprecated)]

// Compiler error in rustc 1.83.0
// Working in rustc 1.84.0-beta.3
// https://github.com/rust-lang/rust/issues/133864

use std::io::Read;
use std::time::Duration;
use std::{env, error::Error};

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

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

use futures::StreamExt;

struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(deafen, join, leave, mute, play, ping, undeafen, unmute, help, report)]
struct General;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("AI_DISCORD_GM_DISCORD_TOKEN")
        .expect("Expected token AI_DISCORD_GM_DISCORD_TOKEN in the environment");

    let framework = StandardFramework::new().group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix("!"));

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        // We insert our own HTTP client here to make use of in
        // `~play`. If we wanted, we could supply cookies and auth
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
            .map_err(|why| println!("Client ended: {:?}", why));
    });

    let _signal_err = tokio::signal::ctrl_c().await;
    println!("Received Ctrl-C, shutting down.");
}

#[command]
#[only_in(guilds)]
async fn deafen(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(&msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_deaf() {
        check_msg(&msg.channel_id.say(&ctx.http, "Already deafened").await);
    } else {
        if let Err(e) = handler.deafen(true).await {
            check_msg(
                &msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(&msg.channel_id.say(&ctx.http, "Deafened").await);
    }

    Ok(())
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
                println!(
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
            check_msg(
                &msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(&msg.channel_id.say(&ctx.http, "Left voice channel").await);
    } else {
        check_msg(&msg.reply(ctx, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn mute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(&msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_mute() {
        check_msg(&msg.channel_id.say(&ctx.http, "Already muted").await);
    } else {
        if let Err(e) = handler.mute(true).await {
            check_msg(
                &msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(&msg.channel_id.say(&ctx.http, "Now muted").await);
    }

    Ok(())
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    check_msg(&msg.channel_id.say(&ctx.http, "Pong!").await);
    Ok(())
}

#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            check_msg(
                &msg.channel_id
                    .say(&ctx.http, "Must provide a URL to a video or audio")
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

        let src = if do_search {
            YoutubeDl::new_search(http_client, url)
        } else {
            YoutubeDl::new(http_client, url)
        };
        let _ = handler.play_input(src.clone().into());

        check_msg(&msg.channel_id.say(&ctx.http, "Playing song").await);
    } else {
        check_msg(
            &msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn undeafen(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.deafen(false).await {
            check_msg(
                &msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(&msg.channel_id.say(&ctx.http, "Undeafened").await);
    } else {
        check_msg(
            &msg.channel_id
                .say(&ctx.http, "Not in a voice channel to undeafen in")
                .await,
        );
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn unmute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.mute(false).await {
            check_msg(
                &msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(&msg.channel_id.say(&ctx.http, "Unmuted").await);
    } else {
        check_msg(
            &msg.channel_id
                .say(&ctx.http, "Not in a voice channel to unmute in")
                .await,
        );
    }

    Ok(())
}

//Custom command to display other commands
#[command]
async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    let help_text = "Available commands:
- !ping: Responds with 'Pong!'
- !join: Joins the voice channel you are currently in.
- !leave: Leaves the current voice channel.
- !play <url/search term>: Plays an audio track from a URL or searches YouTube.
- !mute: Mutes the bot in the voice channel.
- !unmute: Unmutes the bot in the voice channel.
- !deafen: Deafens the bot in the voice channel.
- !undeafen: Undeafens the bot in the voice channel.
- !report <text>: Sends the provided text to the game master assistant and displays the answer. Use reaction to read the response aloud.";

    check_msg(&msg.channel_id.say(&ctx.http, help_text).await);
    Ok(())
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: &SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct AssistantRequest {
    prompt: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct OpenAIMessage {
    role: String,
    content: String,
}
async fn run_completion(req: AssistantRequest) -> Result<String, Box<dyn Error + Send + Sync>> {
    let openai_api_key = env::var("AI_DISCORD_GM_OPENAI_API_KEY")
        .expect("Expected token AI_DISCORD_GM_OPENAI_API_KEY in the environment");
    let thread_id = env::var("AI_DISCORD_GM_OPENAI_THREAD_ID")
        .expect("Expected token AI_DISCORD_GM_OPENAI_THREAD_ID in the environment");
    let assistant_id = env::var("AI_DISCORD_GM_OPENAI_ASSISTANT_ID")
        .expect("Expected token AI_DISCORD_GM_OPENAI_ASSISTANT_ID in the environment");
    let client = reqwest::Client::new();

    // Step 1: Send a message to the thread
    let message_resp = client
        .post(format!(
            "https://api.openai.com/v1/threads/{}/messages",
            thread_id
        ))
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .json(&json!({
            "role": "user",
            "content": req.prompt,
        }))
        .send()
        .await?;

    info!(
        "POST https://api.openai.com/v1/threads/{}/messages {:?}",
        //no json data here, we only care whether status is OK
        thread_id,
        message_resp
    );

    if !message_resp.status().is_success() {
        let err_text = message_resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error sending message: {}", err_text).into());
    }

    // Step 2: Create a run
    let run_resp = client
        .post(format!(
            "https://api.openai.com/v1/threads/{}/runs",
            thread_id
        ))
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .json(&json!({            "assistant_id": assistant_id,        }))
        .send()
        .await?;

    info!(
        "POST https://api.openai.com/v1/threads/{}/runs {:?}",
        thread_id, run_resp
    );
    if !run_resp.status().is_success() {
        // ... (Error handling as before)
        let err_text = run_resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error creating run: {}", err_text).into());
    }
    let run_resp_data: serde_json::Value = run_resp.json().await?;
    info!(
        "POST https://api.openai.com/v1/threads/{}/runs {:?}",
        thread_id, run_resp_data
    );

    let run_id = run_resp_data
        .get("id")
        .and_then(|id| id.as_str())
        .ok_or("No run id found")?;

    // Step 3: Wait for the run to complete
    let mut run_status = String::from("queued"); // or whatever the initial status is
    let mut run_status_data: serde_json::Value = serde_json::Value::Object(serde_json::Map::new());
    while run_status == "queued" || run_status == "in_progress" || run_status == "unknown" {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await; // Check every second

        let run_status_resp = client
            .get(format!(
                "https://api.openai.com/v1/threads/{}/runs/{}/steps",
                thread_id, run_id
            ))
            .header("Authorization", format!("Bearer {}", openai_api_key))
            .header("OpenAI-Beta", "assistants=v2")
            .send()
            .await?;
        if !run_status_resp.status().is_success() {
            // ... (Error handling as before)
            let err_text = run_status_resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Error checking run status: {}", err_text).into());
        }

        run_status_data = run_status_resp.json().await?;
        info!(
            "GET https://api.openai.com/v1/threads/{}/runs/{}/steps {:?}",
            thread_id, run_id, run_status_data
        );

        run_status = run_status_data
            .get("data") // Access the "data" array
            .and_then(|data| data.as_array())
            .and_then(|array| array.first()) // Get the first step (most recent)
            .and_then(|step| step.get("status"))
            .and_then(|status| status.as_str())
            .map(String::from)
            .unwrap_or_else(|| "unknown".to_string());
    }

    if run_status_data.is_null() {
        return Err(format!("Error checking run_status_json").into());
    }

    //FIXME
    info!(
        "DEBUG LAST GET https://api.openai.com/v1/threads/{}/runs/{}/steps {:?} ; run_status={}",
        thread_id, run_id, run_status_data, run_status
    );

    let message_id = run_status_data
        .get("data")
        .and_then(|data| data.as_array())
        .and_then(|array| array.first()) // Get the first step
        .and_then(|step| step.get("step_details"))
        .and_then(|details| details.get("message_creation"))
        .and_then(|msg_creation| msg_creation.get("message_id"))
        .and_then(|id| id.as_str())
        .map(String::from)
        .ok_or(format!(
            "message_id not found in step details, run_status_data={}, run_status={}",
            run_status_data, run_status,
        ))?;

    // Step 4: Retrieve the message (after run completion)
    let message_response: reqwest::Response = client
        .get(format!(
            "https://api.openai.com/v1/threads/{}/messages/{}",
            thread_id, message_id
        ))
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .send()
        .await?;

    if !message_response.status().is_success() {
        let err_text = message_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error retrieving message: {}", err_text).into());
    }

    let message_data: serde_json::Value = message_response.json().await?;
    info!(
        "GET https://api.openai.com/v1/threads/{}/messages/{} {:?}",
        thread_id, message_id, message_data
    );

    let latest_message = message_data
        .get("content")
        .and_then(|content| content.as_array())
        .and_then(|array| array.first())
        .and_then(|content_obj| content_obj.get("text"))
        .and_then(|text_obj| text_obj.get("value"))
        .and_then(|value| value.as_str())
        .ok_or("No message content found")?;

    Ok(latest_message.to_string())
}

//FIXME The maximum length is 4096 characters.
async fn text_to_speech(
    text: &str,
    destination_path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let openai_api_key = env::var("AI_DISCORD_GM_OPENAI_API_KEY")?;
    let client = reqwest::Client::new();

    let response = client
        .post("https://api.openai.com/v1/audio/speech")
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "tts-1-hd", //FIXME try tts-1
            "input": text,
            "response_format": "opus",
            "voice": "onyx",
            "speed": "1.4"
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let err_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error generating speech: {}", err_text).into());
    }

    let mut file = tokio::fs::File::create(destination_path).await?;
    let mut content = response.bytes_stream();

    while let Some(item) = content.next().await {
        let chunk = item?;
        file.write_all(&chunk).await?;
    }

    Ok(())
}

async fn read_local_audio(
    ctx: &Context,
    guild_id: GuildId,
    msg: &Message,
    audio_path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("read_local_audio {}", audio_path);

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

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
        check_msg(&msg.channel_id.say(&ctx.http, error_message).await);
        Err(error_message.into())
    }
}

#[command]
#[only_in(guilds)]
async fn report(ctx: &Context, msg_user: &Message) -> CommandResult {
    let prompt = msg_user.content[8..].to_string();

    match handle_report(ctx, msg_user, prompt).await {
        Ok(_) => Ok(()),
        Err(e) => {
            check_msg(
                &msg_user
                    .channel_id
                    .say(&ctx.http, format!("Error: {}", e))
                    .await,
            );
            Err(e.into()) // Or handle the error differently if needed
        }
    }
}

const EMOJI_SOUND: &str = "🔊";
const EMOJI_TIME: &str = "⏱️";
const EMOJI_WAIT: &str = "⏳";
const EMOJI_DONE: &str = "✅";
fn emoji(e: &str) -> serenity::all::ReactionType {
    return ReactionType::Unicode(e.to_string());
}

fn pre_prompt(user: &serenity::model::user::User) -> &str {
    match user.id.get() {
        607653619122307123 => "Comm dit:",
        374989552646881281 => "Explo dit:",
        3 => "Secu dit:",
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

    let msg_generated = msg_user.channel_id.say(&ctx.http, &text_generated).await?;
    info!("delete_reaction msg_user EMOJI_WAIT",);
    delete_reaction(ctx, msg_user, emoji(EMOJI_WAIT)).await?;
    info!("add_reaction msg_user EMOJI_DONE",);
    add_reaction(ctx, &msg_user, emoji(EMOJI_DONE).clone()).await?;

    let file_path: String = format!("{}{}", ASSETS_DIR, generate_file_hash(&text_generated));
    let text_path = format!("{}.txt", file_path);

    let text_content: String = format!("{}\n---\n{}", prompt2, text_generated);
    std::fs::write(text_path, text_content).unwrap();

    let guild_id = msg_user.guild_id.unwrap();

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

    info!("add_reaction msg EMOJI_SOUND",);
    add_reaction(ctx, &msg, emoji(EMOJI_SOUND)).await?;
    let collector = collect_reaction(ctx, &msg).await;

    if let Some(reaction) = collector {
        info!("delete_reaction msg EMOJI_SOUND",);
        delete_reaction(ctx, &msg, emoji(EMOJI_SOUND)).await?;
        info!("add_reaction msg EMOJI_WAIT",);
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
    }
    info!("delete_reaction msg EMOJI_WAIT",);
    delete_reaction(ctx, &msg, emoji(EMOJI_WAIT)).await?;
    info!("add_reaction msg EMOJI_TIME",);
    add_reaction(ctx, &msg, emoji(EMOJI_TIME)).await?;

    Ok(())
}

async fn add_reaction(
    ctx: &Context,
    msg: &Message,
    reaction: ReactionType,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    msg.react(&ctx.http, reaction).await?;
    Ok(())
}

async fn collect_reaction(
    ctx: &Context,
    msg: &Message,
) -> Option<serenity::model::channel::Reaction> {
    msg.await_reaction(ctx)
        .timeout(Duration::from_secs(600))
        .await
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

use sha2::{Digest, Sha256};

fn generate_file_hash(text_generated: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text_generated.as_bytes());
    let result = hasher.finalize();
    let hash_hex = hex::encode(result);
    hash_hex[..8].to_string()
}
