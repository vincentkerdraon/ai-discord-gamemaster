//FIXME is there a structure+naming conventions for files in crates?
//It seems by default it is named lib.rs
//FIXME Is this a good practice?
mod models;
pub use models::AssistantRequest;
mod run_completion;
pub use run_completion::run_completion;
mod text_to_speech;
pub use text_to_speech::text_to_speech;

use std::error::Error;
use text_completion::RequestHandler;
use tokio::sync::oneshot::{channel, Sender};

pub struct OpenAIHandler;

unsafe impl Sync for OpenAIHandler {}

impl RequestHandler for OpenAIHandler {
    fn answer_request(
        &self,
        request: &str,
        response_sender: Sender<Result<String, Box<dyn Error + Send + Sync>>>,
    ) {
        //FIXME

        let (tx, _) = channel(); //create oneshot channel for async response

        let req = request.to_string();
        tokio::spawn(async move {
            let result = run_completion(AssistantRequest { prompt: req }).await;
            let _ = tx.send(result); // Ignore errors in sending the response
        });

        //This should be done if you don't want to block but don't care about the result either
        let _ = response_sender.send(Ok("".to_string()));
    }

    fn text_to_speech(
        &self,
        text: &str,
        destination_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        //FIXME same as answer_request

        //FIXME what does to_owned do?
        let text = text.to_owned();
        let destination_path = destination_path.to_owned();

        let (tx, _) = channel();
        tokio::spawn(async move {
            let result = text_to_speech(&text, &destination_path).await;
            let _ = tx.send(result);
        });

        Ok(())
    }
}
