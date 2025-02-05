use std::{collections::HashMap, env, sync::Arc};

use openai_api::{OpenAIHandler, PrePromptByUser, TTSConfig};
use tracing::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    info!(
        "Starting... CARGO_PKG_NAME={}, CARGO_PKG_VERSION={}, version={}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        option_env!("version").unwrap_or("(not defined at compile)")
    );

    let discord_token = read_env_var("AI_DISCORD_GM_DISCORD_TOKEN");
    let openai_api_key = read_env_var("AI_DISCORD_GM_OPENAI_API_KEY");
    let thread_id = read_env_var("AI_DISCORD_GM_OPENAI_THREAD_ID");
    let assistant_id = read_env_var("AI_DISCORD_GM_OPENAI_ASSISTANT_ID");

    let openai_handler = Arc::new(OpenAIHandler {
        assistant_id,
        api_key: openai_api_key,
        thread_id,
        tts_config: TTSConfig {
            //should load from a config file
            model: "tts-1".to_string(),
            response_format: "opus".to_string(),
            voice: "onyx".to_string(),
            speed: 1.3,
        },
        pre_prompt_by_user: PrePromptByUser {
            default: "Quelqu'un dit:".to_string(),
            users: HashMap::from([
                (607653619122307123, "Comm dit:".to_string()),
                (374989552646881281, "Explo dit:".to_string()),
                (518896639608619022, "Secu dit:".to_string()),
            ]),
        },
    });
    let reaction_listen_s: std::time::Duration = std::time::Duration::from_secs(600);
    discord::init(&discord_token, reaction_listen_s, openai_handler).await;

    let _signal_err = tokio::signal::ctrl_c().await;
    info!("Received Ctrl-C, shutting down.");

    Ok(())
}

fn read_env_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("Expected env var: {}", name))
}
