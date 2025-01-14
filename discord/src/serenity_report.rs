// stolen from https://github.com/serenity-rs/songbird/blob/current/examples/serenity/voice/src/main.rs
// (I had my own code, but it would fail when joining and I never figured why)
// This is working out of the box
// But with deprecation warnings
#![allow(deprecated)]

use std::error::Error;
use std::time::Duration;
use tracing::{debug, warn};

use serenity::client::Context;
use songbird::id::GuildId;

use serenity::model::{channel::Message, prelude::ReactionType};

use crate::emoji::{add_reaction, delete_reaction, emoji, EMOJI_DONE, EMOJI_SOUND, EMOJI_WAIT};
use crate::serenity_audio::read_local_audio;
use crate::{check_msg, generate_file_hash, DiscordHandler, ASSETS_DIR};

pub async fn handle_report(
    ctx: &Context,
    discord_handler: &DiscordHandler,
    msg_user: &Message,
    mut prompt: String, //FIXME &str?
) -> Result<(), Box<dyn Error + Send + Sync>> {
    ////FIXME make this async, we don't need to wait to keep going
    add_reaction(ctx, &msg_user, emoji(EMOJI_WAIT)).await?;

    //FIXME move pre_prompt to handler
    prompt = format!(
        "{}{}",
        discord_handler
            .request_handler
            .pre_prompt(&msg_user.author.id.get()),
        prompt
    );
    let prompt2 = prompt.clone();

    let (tx, rx) = tokio::sync::oneshot::channel();
    discord_handler.request_handler.answer_request(&prompt, tx);

    let text_generated = match rx.await {
        Ok(result) => match result {
            Ok(text) => text,
            Err(e) => {
                warn!("Error from RequestHandler: {}", e);
                return Err(format!("OpenAI request failed: {}", e).into());
            }
        },
        Err(_) => {
            warn!("Error receiving result from RequestHandler.");
            return Err("OpenAI request failed.".into());
        }
    };

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
        discord_handler,
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
    discord_handler: &DiscordHandler,
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
            discord_handler,
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

async fn handle_reaction(
    ctx: &Context,
    discord_handler: &DiscordHandler,
    guild_id: GuildId,
    msg: &Message,
    text_generated: &str,
    hash_file_name: &str,
    reaction: serenity::model::channel::Reaction,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let ReactionType::Unicode(emoji) = reaction.emoji {
        if emoji != EMOJI_SOUND {
            return Ok(());
        }
        let ctx = ctx.clone();
        let audio_path = format!("{}.opus", hash_file_name);

        let (tx, rx) = tokio::sync::oneshot::channel();
        discord_handler
            .request_handler
            .text_to_speech(text_generated, &audio_path, tx);

        match rx.await {
            Ok(result) => match result {
                Ok(()) => {
                    if let Err(why) = read_local_audio(&ctx, guild_id, msg, &audio_path).await {
                        check_msg(
                            &msg.reply(&ctx.http, &format!("Audio playback failed: {:?}", why))
                                .await,
                        );
                        return Err(why);
                    }
                }
                Err(e) => {
                    check_msg(
                        &msg.reply(&ctx.http, &format!("Text-to-speech failed: {:?}", e))
                            .await,
                    );
                    return Err(e);
                }
            },
            Err(_) => {
                warn!("Error receiving result from RequestHandler.");
                return Err("OpenAI request failed.".into());
            }
        };
    }
    Ok(())
}
