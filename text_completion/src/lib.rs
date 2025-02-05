//! An interface for handling text completion requests
//!
//! This crate provides a `RequestHandler` trait that defines methods for processing
//! text-based requests and converting text to speech.

use std::error::Error;
use tokio::sync::oneshot::Sender;

pub trait RequestHandler {
    //MEETUP032 Is this trait a good practice?
    //I don't want the discord code to be linked to the openai code.
    //I prefer a dependency injection using an abstraction.

    //FIXME can't return `-> Result<(), Box<dyn Error + Send + Sync>>`
    //need to pass the result in a channel provided as param
    //because of syn-something

    fn answer_report(
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

    fn pre_prompt(&self, user_id: &u64) -> &str;
}
