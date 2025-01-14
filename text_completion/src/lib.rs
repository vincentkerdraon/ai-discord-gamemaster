use std::error::Error;
use tokio::sync::oneshot::Sender;

pub trait RequestHandler {
    //FIXME can't use `-> Result<(), Box<dyn Error + Send + Sync>>`
    //need to pass the result in a channel provided as param
    //because of syn-something

    fn answer_request(
        &self,
        request: &str,
        result: Sender<Result<String, Box<dyn Error + Send + Sync>>>,
    );

    fn text_to_speech(
        &self,
        text: &str,
        destination_path: &str,
        result: Sender<Result<(), Box<dyn Error + Send + Sync>>>,
    );
}
