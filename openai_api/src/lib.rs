//! A library for interacting with OpenAI's API.
//!
//! This crate provides an `OpenAIHandler` struct that enables making requests
//! to OpenAI's API for tasks such as text completion and text-to-speech synthesis.
//! It also includes utilities for managing request pre-prompts per user.

//MEETUP031 is there a structure+naming conventions for files in crates?
// It seems by default it is named lib.rs
// Do you use: #![doc(html_root_url = "https://docs.rs/openai_api/0.1.0")]

mod models;
pub use models::AssistantRequest;
mod run_completion;
use reqwest::Response;
pub use run_completion::run_completion;
mod text_to_speech;
pub use text_to_speech::text_to_speech;
mod pre_prompt_by_user;
pub use pre_prompt_by_user::PrePromptByUser;
use tracing::*;

use http::header::{AUTHORIZATION, CONTENT_TYPE};
use std::error::Error;
use text_completion::RequestHandler;
use tokio::sync::oneshot::Sender;

#[derive(Clone)]
pub struct TTSConfig {
    /// example: "tts-1",
    pub model: String,
    /// example: "opus",   
    pub response_format: String,
    /// example: "onyx",
    pub voice: String,
    /// example: 1.3
    pub speed: f64,
}

#[derive(Clone)]
pub struct OpenAIHandler {
    pub api_key: String,
    pub thread_id: String,
    pub assistant_id: String,

    pub tts_config: TTSConfig,
    pub pre_prompt_by_user: PrePromptByUser,
}

//MEETUP011 unsafe=not good?
unsafe impl Sync for OpenAIHandler {}

impl RequestHandler for OpenAIHandler {
    fn answer_report(
        &self,
        request: &str,
        result: Sender<Result<String, Box<dyn Error + Send + Sync>>>,
    ) {
        let req = request.to_string();
        let handler = self.clone();

        tokio::spawn(async move {
            let r = run_completion(&handler, AssistantRequest { prompt: req }).await;
            let _ = result.send(r);
        });
    }

    fn text_to_speech(
        &self,
        text: &str,
        destination_path: &str,
        result: Sender<Result<(), Box<dyn Error + Send + Sync>>>,
    ) {
        let text = text.to_string();
        let destination_path = destination_path.to_string();
        let handler = self.clone();

        tokio::spawn(async move {
            let r: Result<(), Box<dyn Error + Send + Sync>> =
                text_to_speech(&handler, &text, &destination_path).await;
            let _ = result.send(r);
        });
    }

    fn pre_prompt(&self, user_id: &u64) -> &str {
        self.pre_prompt_by_user.prompt(user_id)
    }
}

const API_SPEECH: &str = "https://api.openai.com/v1/audio/speech";
const API_THREAD_MESSAGES: &str = "https://api.openai.com/v1/threads/{}/messages";
const API_THREAD_MESSAGE: &str = "https://api.openai.com/v1/threads/{}/messages/{}";
const API_THREAD_RUNS: &str = "https://api.openai.com/v1/threads/{}/runs";
const API_THREAD_RUNS_STEPS: &str = "https://api.openai.com/v1/threads/{}/runs/{}/steps";
const API_HEADER_BETA: &str = "OpenAI-Beta";
const API_VALUE_BETA: &str = "assistants=v2";
const API_PULL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

//not defined in http::header
const HEADER_BEARER: &str = "Bearer";
const HEADER_JSON: &str = "application/json";

//take a const in arg1 + any number of &str. replace {} in order.
//See std format!()
//avoid `format argument must be a string literal`
fn format_with_const(template: &str, args: &[&str]) -> String {
    let mut formatted = String::from(template);
    for arg in args {
        if let Some(pos) = formatted.find("{}") {
            formatted.replace_range(pos..pos + 2, arg);
        }
    }
    formatted
}

async fn post(
    handler: &OpenAIHandler,
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<Response, Box<dyn Error + Send + Sync>> {
    let resp = client
        .post(url)
        .header(
            AUTHORIZATION,
            format!("{} {}", HEADER_BEARER, handler.api_key),
        )
        .header(CONTENT_TYPE, HEADER_JSON)
        .header(API_HEADER_BETA, API_VALUE_BETA)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("Error POST {}: {}", url, e).into());

    trace!("POST {} {:?}", url, resp);
    resp
}

async fn get(
    handler: &OpenAIHandler,
    client: &reqwest::Client,
    url: &str,
) -> Result<Response, Box<dyn Error + Send + Sync>> {
    let resp = client
        .get(url)
        .header(
            AUTHORIZATION,
            format!("{} {}", HEADER_BEARER, handler.api_key),
        )
        .header(CONTENT_TYPE, HEADER_JSON)
        .header(API_HEADER_BETA, API_VALUE_BETA)
        .send()
        .await
        .map_err(|e| format!("Error POST {}: {}", url, e).into());

    trace!("GET {} {:?}", url, resp);
    resp
}
