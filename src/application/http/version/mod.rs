pub mod dto;

use super::RunningVersion;
use axum::Json;
use axum::extract::State;
use dto::VersionResponse;

pub async fn get_version(
    State(RunningVersion(app_version)): State<RunningVersion>,
) -> Json<VersionResponse> {
    Json(app_version.into())
}

#[cfg(test)]
mod tests;
