use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct AssistantRequest {
    pub prompt: String,
}
