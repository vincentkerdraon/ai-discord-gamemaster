use std::{env, sync::Arc};

use openai_api::OpenAIHandler;
use tracing::info;

#[tokio::main]
//FIXME return?
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let discord_token = env::var("AI_DISCORD_GM_DISCORD_TOKEN")
        .expect("Expected token AI_DISCORD_GM_DISCORD_TOKEN in the environment");
    let openai_api_key = env::var("AI_DISCORD_GM_OPENAI_API_KEY")
        .expect("Expected token AI_DISCORD_GM_OPENAI_API_KEY in the environment");
    let thread_id = env::var("AI_DISCORD_GM_OPENAI_THREAD_ID")
        .expect("Expected token AI_DISCORD_GM_OPENAI_THREAD_ID in the environment");
    let assistant_id = env::var("AI_DISCORD_GM_OPENAI_ASSISTANT_ID")
        .expect("Expected token AI_DISCORD_GM_OPENAI_ASSISTANT_ID in the environment");

    let openai_handler = Arc::new(OpenAIHandler {
        assistant_id,
        api_key: openai_api_key,
        thread_id,
    });
    discord::init(&discord_token, openai_handler).await;

    let _signal_err = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C, shutting down.");

    Ok(())
}
