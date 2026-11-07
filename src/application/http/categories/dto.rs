use crate::domain::category::{Category, CategoryMember, CategoryView, CategoryViews};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    /// Blank creates the category without one.
    #[serde(default)]
    pub ai_prompt: Option<String>,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub playlists: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    /// `""` removes the prompt.
    #[serde(default)]
    pub ai_prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetCategoryMembersRequest {
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub playlists: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CategoryResponse {
    pub id: String,
    pub name: String,
    pub color: String,
    pub ai_prompt: Option<String>,
    pub channels: Vec<String>,
    pub playlists: Vec<String>,
}

impl From<Category> for CategoryResponse {
    fn from(category: Category) -> Self {
        Self {
            id: category.id.as_str().to_string(),
            name: category.name.as_str().to_string(),
            color: category.color.as_str().to_string(),
            ai_prompt: category
                .ai_prompt
                .as_ref()
                .map(|prompt| prompt.as_str().to_string()),
            channels: category
                .members
                .iter()
                .filter_map(|member| match member {
                    CategoryMember::Channel(handle) => Some(handle.as_str().to_string()),
                    CategoryMember::Playlist(_) => None,
                })
                .collect(),
            playlists: category
                .members
                .iter()
                .filter_map(|member| match member {
                    CategoryMember::Channel(_) => None,
                    CategoryMember::Playlist(id) => Some(id.as_str().to_string()),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CategoryListItemResponse {
    pub id: String,
    pub name: String,
    pub color: String,
    pub ai_prompt: Option<String>,
    pub unwatched_count: usize,
    pub ai_verdict_count: u64,
    pub channels: Vec<String>,
    pub playlists: Vec<String>,
}

impl From<CategoryView> for CategoryListItemResponse {
    fn from(category: CategoryView) -> Self {
        Self {
            id: category.id.as_str().to_string(),
            name: category.name.as_str().to_string(),
            color: category.color.as_str().to_string(),
            ai_prompt: category
                .ai_prompt
                .as_ref()
                .map(|prompt| prompt.as_str().to_string()),
            unwatched_count: category.unwatched_count,
            ai_verdict_count: category.ai_verdict_count,
            channels: category
                .channels
                .iter()
                .map(|handle| handle.as_str().to_string())
                .collect(),
            playlists: category
                .playlists
                .iter()
                .map(|id| id.as_str().to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CategoryListResponse {
    pub categories: Vec<CategoryListItemResponse>,
    pub unwatched_total: usize,
}

impl From<CategoryViews> for CategoryListResponse {
    fn from(views: CategoryViews) -> Self {
        Self {
            categories: views
                .categories
                .into_iter()
                .map(CategoryListItemResponse::from)
                .collect(),
            unwatched_total: views.unwatched_total,
        }
    }
}
