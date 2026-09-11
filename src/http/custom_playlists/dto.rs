use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateCustomPlaylistRequest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddVideoRequest {
    pub video: String,
}
