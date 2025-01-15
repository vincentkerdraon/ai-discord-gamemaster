mod reaction;
mod serenity;
mod serenity_audio;
mod serenity_report;
pub use serenity::init;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use text_completion::RequestHandler;

use ::serenity::{all::Message, prelude::TypeMapKey, Result};

use tracing::warn;

pub struct DiscordHandler {
    request_handler: Arc<dyn RequestHandler + Send + Sync>,
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: &Result<Message>) {
    if let Err(why) = result {
        warn!("Error sending message: {:?}", why);
    }
}

const ASSETS_DIR: &str = "assets/";

pub struct HttpKey;

// YtDl requests need an HTTP client to operate -- we'll create and store our own.
impl TypeMapKey for HttpKey {
    type Value = reqwest::Client;
}

fn generate_file_hash(text_generated: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text_generated.as_bytes());
    let result = hasher.finalize();
    let hash_hex = hex::encode(result);
    hash_hex[..8].to_string()
}
