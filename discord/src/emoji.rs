use ::serenity::all::Context;
use std::error::Error;

use ::serenity::{
    all::{Message, ReactionType},
    Result,
};

pub const EMOJI_SOUND: &str = "🔊";
pub const EMOJI_WAIT: &str = "⏳";
pub const EMOJI_DONE: &str = "✅";

pub fn emoji(e: &str) -> ReactionType {
    return ReactionType::Unicode(e.to_string());
}

pub async fn add_reaction(
    ctx: &Context,
    msg: &Message,
    reaction: ReactionType,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    msg.react(&ctx.http, reaction).await?;
    Ok(())
}

pub async fn delete_reaction(
    ctx: &Context,
    msg: &Message,
    reaction: ReactionType,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    msg.delete_reaction(&ctx.http, None, reaction).await?;
    Ok(())
}
