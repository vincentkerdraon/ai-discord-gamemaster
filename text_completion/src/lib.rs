use std::error::Error;
use tokio::sync::oneshot::Sender;

pub trait RequestHandler {
    fn answer_request(
        &self,
        request: &str,
        response_sender: Sender<Result<String, Box<dyn Error + Send + Sync>>>,
    );

    fn text_to_speech(
        &self,
        text: &str,
        destination_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
