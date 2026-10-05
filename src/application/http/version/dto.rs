use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub struct VersionResponse {
    pub version: String,
}
