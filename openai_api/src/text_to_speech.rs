use futures::StreamExt;
use serde_json::json;
use std::error::Error;
use tokio::io::AsyncWriteExt;

use crate::*;

pub async fn text_to_speech(
    handler: &OpenAIHandler,
    text: &str,
    destination_path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    trace!(
        "text_to_speech starting, in {}, for {}",
        destination_path,
        text
    );
    //The maximum length is 4096 characters for tts (openai limitation)
    //The prompt should make sure we don't reach this limit.
    assert!(text.len() <= 4096);

    let client = reqwest::Client::new();

    let url = API_SPEECH;
    let response = post(
        handler,
        &client,
        url,
        &json!({
            "input": text,
            "model": handler.tts_config.model,
            "response_format": handler.tts_config.response_format,
            "voice": handler.tts_config.voice,
            "speed": handler.tts_config.speed,
        }),
    )
    .await?;

    if !response.status().is_success() {
        let err_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Error generating speech: {}", err_text).into());
    }

    let mut file = match tokio::fs::File::create(destination_path).await {
        Ok(f) => f,
        Err(e) => {
            return Err(format!("File creation error: {}, {}", destination_path, e).into());
        }
    };
    let mut content = response.bytes_stream();
    while let Some(item) = content.next().await {
        match item {
            Ok(chunk) => {
                if let Err(e) = file.write_all(&chunk).await {
                    return Err(format!("File write error: {}, {}", destination_path, e).into());
                }
            }
            Err(e) => {
                return Err(format!("Stream read error: {}", e).into());
            }
        }
    }

    debug!(
        "text_to_speech ready, in {}, for {}",
        destination_path, text
    );
    Ok(())
}
