use futures::StreamExt;
use serde_json::json;
use std::error::Error;
use tokio::io::AsyncWriteExt;

use crate::OpenAIHandler;

//The maximum length is 4096 characters.
pub async fn text_to_speech(
    handler: &OpenAIHandler,
    text: &str,
    destination_path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = reqwest::Client::new();

    let response = client
        .post("https://api.openai.com/v1/audio/speech")
        .header("Authorization", format!("Bearer {}", handler.api_key))
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
