use serde_json::json;
use std::error::Error;
use tracing::*;

use crate::{AssistantRequest, OpenAIHandler};

pub async fn run_completion(
    handler: &OpenAIHandler,
    req: AssistantRequest,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    trace!("run_completion prompt={}", req.prompt);

    let client = reqwest::Client::new();

    // Step 1: Send a message to the thread
    let message_resp = client
        .post(format!(
            "https://api.openai.com/v1/threads/{}/messages",
            handler.thread_id
        ))
        .header("Authorization", format!("Bearer {}", handler.api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .json(&json!({
            "role": "user",
            "content": req.prompt,
        }))
        .send()
        .await?;

    trace!(
        "POST https://api.openai.com/v1/threads/{}/messages {:?}",
        //no json data here, we only care whether status is OK
        handler.thread_id,
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
            handler.thread_id
        ))
        //FIXME is there a const in the std lib with Authorization or Bearer?
        .header("Authorization", format!("Bearer {}", handler.api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .json(&json!({            "assistant_id": handler.assistant_id     }))
        .send()
        .await?;

    trace!(
        "POST https://api.openai.com/v1/threads/{}/runs {:?}",
        handler.thread_id,
        run_resp
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
    trace!(
        "POST https://api.openai.com/v1/threads/{}/runs {:?}",
        handler.thread_id,
        run_resp_data
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
                handler.thread_id, run_id
            ))
            .header("Authorization", format!("Bearer {}", handler.api_key))
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
        trace!(
            "GET https://api.openai.com/v1/threads/{}/runs/{}/steps {:?}",
            handler.thread_id,
            run_id,
            run_status_data
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
        return Err("Error checking run_status_json".into());
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
            handler.thread_id, message_id
        ))
        .header("Authorization", format!("Bearer {}", handler.api_key))
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
    trace!(
        "GET https://api.openai.com/v1/threads/{}/messages/{} {:?}",
        handler.thread_id,
        message_id,
        message_data
    );

    let latest_message = message_data
        .get("content")
        .and_then(|content| content.as_array())
        .and_then(|array| array.first())
        .and_then(|content_obj| content_obj.get("text"))
        .and_then(|text_obj| text_obj.get("value"))
        .and_then(|value| value.as_str())
        .ok_or("No message content found")?;

    debug!(
        "run_completion prompt={} result={}",
        req.prompt, latest_message
    );
    Ok(latest_message.to_string())
}
