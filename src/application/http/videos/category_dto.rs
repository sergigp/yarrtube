use crate::domain::video_category::{VideoCategoryChoice, VideoCategoryChoices};
use serde::{Deserialize, Serialize};

pub const INVALID_BELONGS: &str = "belongs must be true or false";

#[derive(Debug, Deserialize)]
pub struct SetVideoCategoryRequest {
    /// Anything but a boolean is rejected with `INVALID_BELONGS`.
    #[serde(default)]
    pub belongs: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct VideoCategoryChoiceResponse {
    pub id: String,
    pub name: String,
    pub color: String,
    pub belongs: bool,
    pub from: Option<String>,
    pub verdict: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct VideoCategoriesResponse {
    pub categorization_enabled: bool,
    pub categories: Vec<VideoCategoryChoiceResponse>,
}

impl From<VideoCategoryChoice> for VideoCategoryChoiceResponse {
    fn from(choice: VideoCategoryChoice) -> Self {
        Self {
            id: choice.category.id.as_str().to_string(),
            name: choice.category.name.as_str().to_string(),
            color: choice.category.color.as_str().to_string(),
            belongs: choice.belongs,
            from: choice.from.map(|from| from.as_str().to_string()),
            verdict: choice
                .verdict
                .map(|verdict| verdict.as_str().to_lowercase()),
        }
    }
}

impl From<VideoCategoryChoices> for VideoCategoriesResponse {
    fn from(choices: VideoCategoryChoices) -> Self {
        Self {
            categorization_enabled: choices.categorization_enabled,
            categories: choices
                .choices
                .into_iter()
                .map(VideoCategoryChoiceResponse::from)
                .collect(),
        }
    }
}
