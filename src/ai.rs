use crate::errors::AppError;
use serde::{Deserialize, Serialize};

/// AI configuration from config file
#[derive(Debug, Clone, Deserialize)]
pub struct AiConfig {
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    pub model_id: String,
}

impl AiConfig {
    /// Post-process config after deserialization (no env overrides; rely on config file)
    pub fn finalize(self) -> Self {
        self
    }

    /// Check if API key is set
    pub fn has_api_key(&self) -> bool {
        !self.api_key.is_empty()
    }
}

/// LLM API request/response structures
#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

/// AI client for calling LLM APIs
pub struct AiClient {
    config: AiConfig,
}

impl AiClient {
    /// Create a new AI client
    pub fn new(config: AiConfig) -> Result<Self, AppError> {
        if !config.has_api_key() {
            return Err(AppError::Config(crate::errors::ConfigError::ApiKeyMissing.into()));
        }

        Ok(Self { config })
    }

    /// Generate commit message using LLM
    pub async fn generate_commit_message(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, AppError> {
        let client = reqwest::Client::new();
        
        let url = format!("{}/chat/completions", self.config.base_url);
        
        let request = ChatRequest {
            model: self.config.model_id.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            temperature: 0.7,
        };

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::HttpError(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::HttpError(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| AppError::HttpError(format!("Failed to parse response: {}", e)))?;

        if chat_response.choices.is_empty() {
            return Err(AppError::HttpError("No response from LLM".to_string()));
        }

        let message = chat_response.choices[0].message.content.trim().to_string();
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ai_config_has_api_key() {
        let config = AiConfig {
            base_url: "https://api.example.com".to_string(),
            api_key: "test-key".to_string(),
            model_id: "test-model".to_string(),
        };
        assert!(config.has_api_key());

        let config_empty = AiConfig {
            base_url: "https://api.example.com".to_string(),
            api_key: String::new(),
            model_id: "test-model".to_string(),
        };
        assert!(!config_empty.has_api_key());
    }

    #[test]
    fn test_ai_client_new_with_api_key() {
        let config = AiConfig {
            base_url: "https://api.example.com".to_string(),
            api_key: "test-key".to_string(),
            model_id: "test-model".to_string(),
        };
        let client = AiClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_ai_client_new_without_api_key() {
        let config = AiConfig {
            base_url: "https://api.example.com".to_string(),
            api_key: String::new(),
            model_id: "test-model".to_string(),
        };
        let result = AiClient::new(config);
        assert!(result.is_err());
        if let Err(AppError::Config(crate::errors::ConfigError::ApiKeyMissing)) = result {
            // Correct error type
        } else {
            panic!("Expected ApiKeyMissing error");
        }
    }

    #[tokio::test]
    async fn test_ai_client_generate_commit_message() {
        use wiremock::{MockServer, Mock, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "feat: add new feature"
                }
            }]
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&mock_response)
            )
            .mount(&mock_server)
            .await;

        let config = AiConfig {
            base_url: mock_server.uri(),
            api_key: "test-key".to_string(),
            model_id: "test-model".to_string(),
        };

        let client = AiClient::new(config).unwrap();
        let result = client
            .generate_commit_message("system prompt", "user prompt")
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "feat: add new feature");
    }

    #[tokio::test]
    async fn test_ai_client_generate_commit_message_error() {
        use wiremock::{MockServer, Mock, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_string("Internal Server Error")
            )
            .mount(&mock_server)
            .await;

        let config = AiConfig {
            base_url: mock_server.uri(),
            api_key: "test-key".to_string(),
            model_id: "test-model".to_string(),
        };

        let client = AiClient::new(config).unwrap();
        let result = client
            .generate_commit_message("system prompt", "user prompt")
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::HttpError(_)));
    }

    #[tokio::test]
    async fn test_ai_client_generate_commit_message_empty_choices() {
        use wiremock::{MockServer, Mock, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "choices": []
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&mock_response)
            )
            .mount(&mock_server)
            .await;

        let config = AiConfig {
            base_url: mock_server.uri(),
            api_key: "test-key".to_string(),
            model_id: "test-model".to_string(),
        };

        let client = AiClient::new(config).unwrap();
        let result = client
            .generate_commit_message("system prompt", "user prompt")
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::HttpError(_)));
    }

    #[tokio::test]
    async fn test_ai_client_generate_commit_message_invalid_json() {
        use wiremock::{MockServer, Mock, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("not-json")
            )
            .mount(&mock_server)
            .await;

        let config = AiConfig {
            base_url: mock_server.uri(),
            api_key: "test-key".to_string(),
            model_id: "test-model".to_string(),
        };

        let client = AiClient::new(config).unwrap();
        let result = client
            .generate_commit_message("system prompt", "user prompt")
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::HttpError(_)));
    }

    #[tokio::test]
    async fn test_ai_client_generate_commit_message_first_choice_trimmed() {
        use wiremock::{MockServer, Mock, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "  feat: spaced content  "
                    }
                },
                {
                    "message": {
                        "role": "assistant",
                        "content": "should be ignored"
                    }
                }
            ]
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&mock_response)
            )
            .mount(&mock_server)
            .await;

        let config = AiConfig {
            base_url: mock_server.uri(),
            api_key: "test-key".to_string(),
            model_id: "test-model".to_string(),
        };

        let client = AiClient::new(config).unwrap();
        let result = client
            .generate_commit_message("system prompt", "user prompt")
            .await
            .unwrap();

        assert_eq!(result, "feat: spaced content");
    }
}
