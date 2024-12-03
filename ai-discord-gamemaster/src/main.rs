use axum::{
    extract::Json,
    response::Html,
    routing::{get, post},
    Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
    Client as SerenityClient,
};
use std::{env, error::Error};
use tracing::{error, info};

#[derive(Deserialize, Serialize, Debug)]
struct AssistantRequest {
    prompt: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct OpenAIMessage {
    role: String,
    content: String,
}
async fn run_completion(req: AssistantRequest) -> Result<String, Box<dyn Error>> {
    let openai_api_key = env::var("AI_DISCORD_GM_OPENAI_API_KEY")?;
    let thread_id = env::var("AI_DISCORD_GM_OPENAI_THREAD_ID")?;
    let client = Client::new();

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

    info!("{:?}", message_resp);
    if !message_resp.status().is_success() {
        let err_text = message_resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error sending message: {}", err_text).into());
    }

    // Step 2: Retrieve the latest message from the assistant
    let messages_resp = client
        .get(format!(
            "https://api.openai.com/v1/threads/{}/messages",
            thread_id
        ))
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .send()
        .await?;

    info!("{:?}", messages_resp);
    if !messages_resp.status().is_success() {
        let err_text = messages_resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error retrieving messages: {}", err_text).into());
    }

    let messages_data: serde_json::Value = messages_resp.json().await?;
    let latest_message = messages_data
        .get("data")
        .and_then(|data| data.as_array())
        .and_then(|array| array.last())
        .and_then(|msg| msg.get("content"))
        .and_then(|content| content.as_array())
        .and_then(|array| array.first())
        .and_then(|content_obj| content_obj.get("text"))
        .and_then(|text_obj| text_obj.get("value"))
        .and_then(|value| value.as_str())
        .ok_or("No assistant message found")?;

    Ok(latest_message.to_string())
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        info!("Message: {:?}", msg);
        if msg.content == "!hello" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "world!").await {
                println!("Error sending message: {:?}", why);
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let token = env::var("AI_DISCORD_GM_DISCORD_TOKEN")?;
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    info!(token);

    let mut client = SerenityClient::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    let _axum_handle = tokio::spawn(async move {
        info!("Starting server...");

        let app = Router::new()
            .route("/", get(hello_world))
            .route("/completion", post(completion));

        axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }

    Ok(())
}

//Existing functions
async fn hello_world() -> Html<&'static str> {
    Html("Hello, World!")
}

async fn completion(Json(payload): Json<AssistantRequest>) -> String {
    info!("completion receive: {:?}", payload);

    match run_completion(payload).await {
        Ok(res) => res,
        Err(e) => {
            error!("completion {:?}", e);
            format!("error {:?}", e)
        }
    }
}
