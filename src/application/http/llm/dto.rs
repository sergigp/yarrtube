use crate::domain::llm_server::{LlmModel, LlmServer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
pub struct OllamaQuery {
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct OllamaResponse {
    pub version: String,
    pub models: Vec<OllamaModelResponse>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct OllamaModelResponse {
    pub name: String,
    pub size_bytes: u64,
    pub thinking: bool,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct AnthropicResponse {
    pub models: Vec<AnthropicModelResponse>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct AnthropicModelResponse {
    pub name: String,
    pub display_name: String,
    pub context_window: u32,
}

impl From<LlmServer> for OllamaResponse {
    fn from(server: LlmServer) -> Self {
        Self {
            version: server.version.unwrap_or_default(),
            models: server
                .models
                .into_iter()
                .map(OllamaModelResponse::from)
                .collect(),
        }
    }
}

impl From<LlmModel> for OllamaModelResponse {
    fn from(model: LlmModel) -> Self {
        Self {
            name: model.name,
            size_bytes: model.size_bytes.unwrap_or_default(),
            thinking: model.thinking,
        }
    }
}

impl From<LlmServer> for AnthropicResponse {
    fn from(server: LlmServer) -> Self {
        Self {
            models: server
                .models
                .into_iter()
                .map(AnthropicModelResponse::from)
                .collect(),
        }
    }
}

impl From<LlmModel> for AnthropicModelResponse {
    fn from(model: LlmModel) -> Self {
        Self {
            display_name: model.display_name.unwrap_or_else(|| model.name.clone()),
            name: model.name,
            context_window: model.context_window.unwrap_or_default(),
        }
    }
}
