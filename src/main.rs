use std::{env, sync::Arc};

use openai_api::OpenAIHandler;
use tracing::*;

#[tokio::main]
//FIXME return?
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    info!("Starting...");

    let discord_token = read_env_var("AI_DISCORD_GM_DISCORD_TOKEN");
    let openai_api_key = read_env_var("AI_DISCORD_GM_OPENAI_API_KEY");
    let thread_id = read_env_var("AI_DISCORD_GM_OPENAI_THREAD_ID");
    let assistant_id = read_env_var("AI_DISCORD_GM_OPENAI_ASSISTANT_ID");

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

fn read_env_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("Expected env var: {}", name))
}
