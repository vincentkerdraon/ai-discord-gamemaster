//FIXME is there a structure+naming conventions for files in crates?
//It seems by default it is named lib.rs
//FIXME Is this a good practice?
mod models;
pub use models::AssistantRequest;
mod run_completion;
pub use run_completion::run_completion;
mod text_to_speech;
pub use text_to_speech::text_to_speech;
use tracing::warn;

use std::error::Error;
use text_completion::RequestHandler;
use tokio::sync::oneshot::Sender;

pub struct OpenAIHandler;

unsafe impl Sync for OpenAIHandler {}

impl RequestHandler for OpenAIHandler {
    fn answer_request(
        &self,
        request: &str,
        result: Sender<Result<String, Box<dyn Error + Send + Sync>>>,
    ) {
        let req = request.to_string();
        tokio::spawn(async move {
            let r = run_completion(AssistantRequest { prompt: req }).await;
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

        tokio::spawn(async move {
            let r: Result<(), Box<dyn Error + Send + Sync>> =
                text_to_speech(&text, &destination_path)
                    .await
                    .map_err(|e| e.into());
            let _ = result.send(r);
        });
    }

    fn pre_prompt(&self, user_id: &u64) -> &str {
        //FIXME use configuration
        match user_id {
            607653619122307123 => "Comm dit:",
            374989552646881281 => "Explo dit:",
            518896639608619022 => "Secu dit:",
            _ => {
                warn!("Unknown discord user id={}", user_id);
                return "Quelqu'un dit:";
            }
        }
    }
}
