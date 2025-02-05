// stolen from https://github.com/serenity-rs/songbird/blob/current/examples/serenity/voice/src/main.rs
// (I had my own code, but it would fail when joining and I never figured why)
// This is working out of the box
// But with deprecation warnings
#![allow(deprecated)]

use std::error::Error;
use tracing::*;

use serenity::client::Context;
use serenity::model::{channel::Message, prelude::ReactionType};
use songbird::id::GuildId;

use crate::reaction::{add_reaction, delete_reaction, emoji, EMOJI_DONE, EMOJI_SOUND, EMOJI_WAIT};
use crate::serenity_audio::read_local_audio;
use crate::{check_msg, generate_file_hash, DiscordHandler, ASSETS_DIR};

pub async fn handle_report(
    ctx: &Context,
    discord_handler: &DiscordHandler,
    msg_user: &Message,
    prompt: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    //we don't need to wait to keep going, we can ignore any error in this case.
    //MEETUP030 is tokio::spawn() creating a real thread or more like a goroutine?
    //Alternative to tokio for this kind of async?
    //
    // add_reaction(ctx, msg_user, emoji(EMOJI_WAIT)).await?;
    let ctx_clone = ctx.clone();
    let msg_user_clone = msg_user.clone();
    tokio::spawn(async move {
        if let Err(e) = add_reaction(&ctx_clone, &msg_user_clone, emoji(EMOJI_WAIT)).await {
            //Not critical if it fails, keep going.
            warn!("Failed to add reaction: {:?}", e);
        }
    });

    let pre_prompt = discord_handler
        .request_handler
        .pre_prompt(&msg_user.author.id.get());

    let prompt = format!("{}{}", pre_prompt, prompt);

    let (tx, rx) = tokio::sync::oneshot::channel();
    discord_handler.request_handler.answer_report(&prompt, tx);

    let text_generated = match rx.await {
        Ok(result) => match result {
            Ok(text) => text,
            Err(e) => {
                return Err(format!("answer_report failed: {}", e).into());
            }
        },
        Err(_) => {
            return Err("answer_report failed".into());
        }
    };

    let msg_generated = msg_user.reply(&ctx.http, &text_generated).await?;

    if let Err(why) = delete_reaction(ctx, msg_user, emoji(EMOJI_WAIT)).await {
        //Not critical if it fails, keep going.
        warn!("Failed to delete reaction: {}", why);
    }
    add_reaction(ctx, msg_user, emoji(EMOJI_DONE).clone()).await?;
    add_reaction(ctx, &msg_generated, emoji(EMOJI_SOUND)).await?;

    let file_path: String = format!("{}{}", ASSETS_DIR, generate_file_hash(&text_generated));
    let text_path = format!("{}.txt", file_path);

    let text_content: String = format!("{}\n---\n{}", &prompt, text_generated);
    //Create asset dir if needed
    std::fs::create_dir_all(ASSETS_DIR)?;
    std::fs::write(text_path, text_content).unwrap();

    let guild_id = msg_user.guild_id.unwrap();

    //detect if the reaction is clicked
    //also filters out the bot to avoid detecting own reaction
    react_and_handle_response(
        ctx,
        discord_handler,
        guild_id.into(),
        msg_generated,
        text_generated,
        &file_path,
    )
    .await?;
    Ok(())
}

/// React when the emoji is clicked the first time. With timeout.
async fn react_and_handle_response(
    ctx: &Context,
    discord_handler: &DiscordHandler,
    guild_id: GuildId,
    msg: Message,
    text_generated: String,
    hash_file_name: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    loop {
        //Run a ReactionCollector
        let reaction = msg
            .await_reaction(ctx)
            .timeout(discord_handler.reaction_listen_s)
            .await;

        //Timeout listening
        if reaction.is_none() {
            trace!("Stop listening for reactions");
            return Ok(());
        }
        let reaction = reaction.unwrap();

        if reaction.emoji != emoji(EMOJI_SOUND) {
            continue;
        }
        //not interested in bot own reactions
        if let Some(ref member) = reaction.member {
            if member.user.bot {
                continue;
            }
        }

        delete_reaction(ctx, &msg, emoji(EMOJI_SOUND)).await?;
        if let Err(why) = add_reaction(ctx, &msg, emoji(EMOJI_WAIT)).await {
            warn!("Fail add reaction: {}", why);
        }

        handle_reaction(
            ctx,
            discord_handler,
            guild_id,
            &msg,
            &text_generated,
            hash_file_name,
            reaction,
        )
        .await?;

        if let Err(why) = delete_reaction(ctx, &msg, emoji(EMOJI_WAIT)).await {
            warn!("Fail delete reaction: {}", why);
        }
        if let Err(why) = add_reaction(ctx, &msg, emoji(EMOJI_DONE)).await {
            warn!("Fail add reaction: {}", why);
        }

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
                return Err("Text to speech failed".into());
            }
        };
    }
    Ok(())
}
