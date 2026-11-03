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

impl From<LlmServer> for OllamaResponse {
    fn from(server: LlmServer) -> Self {
        Self {
            version: server.version,
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
            size_bytes: model.size_bytes,
            thinking: model.thinking,
        }
    }
}
