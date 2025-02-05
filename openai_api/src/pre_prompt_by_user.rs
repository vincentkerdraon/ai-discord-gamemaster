use std::collections::HashMap;
use tracing::warn;

#[derive(Clone)]
pub struct PrePromptByUser {
    pub default: String,
    pub users: HashMap<u64, String>,
}

impl PrePromptByUser {
    pub fn prompt(&self, user_id: &u64) -> &str {
        self.users.get(user_id).map_or_else(
            || {
                warn!("Unknown discord user id={}", user_id);
                &self.default
            },
            |message| message,
        )
    }
}

//(This is a very stupid test, but I am learning)

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_prompt_known_user() {
        let mut users = HashMap::new();
        users.insert(12345, "Hello, user 12345!".to_string());

        let pre_prompt = PrePromptByUser {
            default: "Default prompt".to_string(),
            users,
        };

        assert_eq!(pre_prompt.prompt(&12345), "Hello, user 12345!");
    }

    #[test]
    fn test_prompt_unknown_user() {
        let pre_prompt = PrePromptByUser {
            default: "Default prompt".to_string(),
            users: HashMap::new(),
        };

        assert_eq!(pre_prompt.prompt(&99999), "Default prompt");
    }
}
