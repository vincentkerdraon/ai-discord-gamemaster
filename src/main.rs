use std::{env, sync::Arc};

use openai_api::OpenAIHandler;
use tracing::info;

#[tokio::main]
//FIXME return?
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let token = env::var("AI_DISCORD_GM_DISCORD_TOKEN")
        .expect("Expected token AI_DISCORD_GM_DISCORD_TOKEN in the environment");

    let openai_handler = Arc::new(OpenAIHandler);
    discord::init(&token, openai_handler).await;

    let _signal_err = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C, shutting down.");

    Ok(())
}
