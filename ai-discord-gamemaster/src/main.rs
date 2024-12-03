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
    let assistant_id = env::var("AI_DISCORD_GM_OPENAI_ASSISTANT_ID")?;
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

    info!(
        "POST https://api.openai.com/v1/threads/{}/messages {:?}",
        //no json data here, we only care whether status is OK
        thread_id,
        message_resp
    );

    if !message_resp.status().is_success() {
        let err_text = message_resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error sending message: {}", err_text).into());
    }

    // Step 2: Create a run
    let run_resp = client
        .post(format!(
            "https://api.openai.com/v1/threads/{}/runs",
            thread_id
        ))
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .json(&json!({            "assistant_id": assistant_id,        }))
        .send()
        .await?;

    info!(
        "POST https://api.openai.com/v1/threads/{}/runs {:?}",
        thread_id, run_resp
    );
    if !run_resp.status().is_success() {
        // ... (Error handling as before)
        let err_text = run_resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error creating run: {}", err_text).into());
    }
    let run_resp_data: serde_json::Value = run_resp.json().await?;
    info!(
        "POST https://api.openai.com/v1/threads/{}/runs {:?}",
        thread_id, run_resp_data
    );

    let run_id = run_resp_data
        .get("id")
        .and_then(|id| id.as_str())
        .ok_or("No run id found")?;

    // Step 3: Wait for the run to complete
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let mut run_status = String::from("queued"); // or whatever the initial status is
    let mut run_status_data: serde_json::Value = serde_json::Value::Object(serde_json::Map::new());
    while run_status == "queued" || run_status == "in_progress" {
        let run_status_resp = client
            .get(format!(
                "https://api.openai.com/v1/threads/{}/runs/{}/steps",
                thread_id, run_id
            ))
            .header("Authorization", format!("Bearer {}", openai_api_key))
            .header("OpenAI-Beta", "assistants=v2")
            .send()
            .await?;
        if !run_status_resp.status().is_success() {
            // ... (Error handling as before)
            let err_text = run_status_resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Error checking run status: {}", err_text).into());
        }

        run_status_data = run_status_resp.json().await?;
        info!(
            "POST https://api.openai.com/v1/threads/{}/runs/{}/steps {:?}",
            thread_id, run_id, run_status_data
        );

        run_status = run_status_data
            .get("data") // Access the "data" array
            .and_then(|data| data.as_array())
            .and_then(|array| array.first()) // Get the first step (most recent)
            .and_then(|step| step.get("status"))
            .and_then(|status| status.as_str())
            .map(String::from)
            .unwrap_or_else(|| "unknown".to_string());

        tokio::time::sleep(std::time::Duration::from_secs(1)).await; // Check every second
    }

    if run_status_data.is_null() {
        return Err(format!("Error checking run_status_json").into());
    }

    let message_id = run_status_data
        .get("data")
        .and_then(|data| data.as_array())
        .and_then(|array| array.first()) // Get the first step
        .and_then(|step| step.get("step_details"))
        .and_then(|details| details.get("message_creation"))
        .and_then(|msg_creation| msg_creation.get("message_id"))
        .and_then(|id| id.as_str())
        .map(String::from)
        .ok_or("message_id not found in step details")?;

    // Step 4: Retrieve the message (after run completion)
    let message_response = client
        .get(format!(
            "https://api.openai.com/v1/threads/{}/messages/{}",
            thread_id, message_id
        ))
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .send()
        .await?;

    if !message_response.status().is_success() {
        let err_text = message_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error retrieving message: {}", err_text).into());
    }

    let message_data: serde_json::Value = message_response.json().await?;
    info!(
        "https://api.openai.com/v1/threads/{}/messages/{} {:?}",
        thread_id, message_id, message_data
    );

    let latest_message = message_data
        .get("content")
        .and_then(|content| content.as_array())
        .and_then(|array| array.first())
        .and_then(|content_obj| content_obj.get("text"))
        .and_then(|text_obj| text_obj.get("value"))
        .and_then(|value| value.as_str())
        .ok_or("No message content found")?;

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
