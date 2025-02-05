use serde_json::json;
use std::error::Error;
use tracing::*;

use crate::*;

pub async fn run_completion(
    handler: &OpenAIHandler,
    req: AssistantRequest,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    trace!("run_completion prompt={}", req.prompt);
    let client = reqwest::Client::new();

    // Step 1: Send a message to the thread
    //MEETUP020 I want to use format!() with const!
    // let url = format!(API_THREAD_MESSAGES, handler.thread_id);
    let url = format_with_const(API_THREAD_MESSAGES, &[&handler.thread_id]);
    let message_resp = post(
        handler,
        &client,
        &url,
        &json!({
        "role": "user",
        "content": req.prompt,
        }),
    )
    .await?;

    if !message_resp.status().is_success() {
        let err_text = message_resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error sending message: {}", err_text).into());
    }

    // Step 2: Create a run
    let url = format_with_const(API_THREAD_RUNS, &[&handler.thread_id]);
    let run_resp = post(
        handler,
        &client,
        &url,
        &json!({"assistant_id": handler.assistant_id}),
    )
    .await?;

    if !run_resp.status().is_success() {
        let err_text = run_resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error creating run: {}", err_text).into());
    }
    let run_resp_data: serde_json::Value = run_resp.json().await?;
    trace!("POST {} {:?}", url, run_resp_data);

    let run_id = run_resp_data
        .get("id")
        .and_then(|id| id.as_str())
        .ok_or("No run id found")?;

    // Step 3: Wait for the run to complete
    //MEETUP021 good practice to reuse variable in loop?
    let mut run_status;
    let mut run_status_data: serde_json::Value;
    loop {
        //MEETUP022 better than sleep? like with go context?
        tokio::time::sleep(API_PULL_INTERVAL).await;

        let url = format_with_const(API_THREAD_RUNS_STEPS, &[&handler.thread_id, run_id]);
        let run_status_resp = get(handler, &client, &url).await?;
        if !run_status_resp.status().is_success() {
            let err_text = run_status_resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Error checking run status: {}", err_text).into());
        }

        run_status_data = run_status_resp.json().await?;
        trace!("GET {} {:?}", url, run_status_data);

        run_status = run_status_data
            .get("data") // Access the "data" array
            .and_then(|data| data.as_array())
            .and_then(|array| array.first()) // Get the first step (most recent)
            .and_then(|step| step.get("status"))
            .and_then(|status| status.as_str())
            .map(String::from)
            .unwrap_or_else(|| "unknown".to_string());

        if run_status == "completed" {
            break;
        } else if run_status == "failed" {
            return Err("Run failed".into());
        }
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
    let url = format_with_const(API_THREAD_MESSAGE, &[&handler.thread_id, &message_id]);
    let message_response = get(handler, &client, &url).await?;

    if !message_response.status().is_success() {
        let err_text = message_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error retrieving message: {}", err_text).into());
    }

    let message_data: serde_json::Value = message_response.json().await?;
    trace!("GET {} {:?}", url, message_data);

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
