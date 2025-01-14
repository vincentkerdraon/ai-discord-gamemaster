use std::error::Error;
use std::io::Read;
use tracing::debug;

use songbird::id::GuildId;

use songbird::input::Input;

// Import the `Context` to handle commands.
use serenity::client::Context;

use serenity::model::channel::Message;

use crate::check_msg;

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
