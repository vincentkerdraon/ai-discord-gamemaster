//FIXME is there a structure+naming conventions for files in crates?
//It seems by default it is named lib.rs
//FIXME Is this a good practice?
mod models;
pub use models::AssistantRequest;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, error::Error};
use tokio::io::AsyncWriteExt;
use tracing::debug;

#[derive(Deserialize, Serialize, Debug)]
struct OpenAIMessage {
    role: String,
    content: String,
}
pub async fn run_completion(req: AssistantRequest) -> Result<String, Box<dyn Error + Send + Sync>> {
    let openai_api_key = env::var("AI_DISCORD_GM_OPENAI_API_KEY")
        .expect("Expected token AI_DISCORD_GM_OPENAI_API_KEY in the environment");
    let thread_id = env::var("AI_DISCORD_GM_OPENAI_THREAD_ID")
        .expect("Expected token AI_DISCORD_GM_OPENAI_THREAD_ID in the environment");
    let assistant_id = env::var("AI_DISCORD_GM_OPENAI_ASSISTANT_ID")
        .expect("Expected token AI_DISCORD_GM_OPENAI_ASSISTANT_ID in the environment");
    let client = reqwest::Client::new();

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

    debug!(
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

    debug!(
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
    debug!(
        "POST https://api.openai.com/v1/threads/{}/runs {:?}",
        thread_id, run_resp_data
    );

    let run_id = run_resp_data
        .get("id")
        .and_then(|id| id.as_str())
        .ok_or("No run id found")?;

    // Step 3: Wait for the run to complete
    let mut run_status = String::from("queued"); // or whatever the initial status is
    let mut run_status_data: serde_json::Value = serde_json::Value::Object(serde_json::Map::new());
    while run_status == "queued" || run_status == "in_progress" || run_status == "unknown" {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await; // Check every second

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
        debug!(
            "GET https://api.openai.com/v1/threads/{}/runs/{}/steps {:?}",
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
        .ok_or(format!(
            "message_id not found in step details, run_status_data={}, run_status={}",
            run_status_data, run_status,
        ))?;

    // Step 4: Retrieve the message (after run completion)
    let message_response: reqwest::Response = client
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
    debug!(
        "GET https://api.openai.com/v1/threads/{}/messages/{} {:?}",
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

//The maximum length is 4096 characters.
pub async fn text_to_speech(
    text: &str,
    destination_path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let openai_api_key = env::var("AI_DISCORD_GM_OPENAI_API_KEY")?;
    let client = reqwest::Client::new();

    let response = client
        .post("https://api.openai.com/v1/audio/speech")
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "tts-1",
            "input": text,
            "response_format": "opus",
            "voice": "onyx",
            "speed": "1.3"
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        let err_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error generating speech: {}", err_text).into());
    }

    let mut file = tokio::fs::File::create(destination_path).await?;
    let mut content = response.bytes_stream();

    while let Some(item) = content.next().await {
        let chunk = item?;
        file.write_all(&chunk).await?;
    }

    Ok(())
}
