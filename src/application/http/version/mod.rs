pub mod dto;

use super::RunningVersion;
use axum::Json;
use axum::extract::State;
use dto::VersionResponse;

pub async fn get_version(
    State(RunningVersion(version)): State<RunningVersion>,
) -> Json<VersionResponse> {
    Json(VersionResponse { version })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_should_return_the_running_version() {
        let running_version = RunningVersion("v0.9.2".to_string());

        let response = get_version(State(running_version)).await;

        assert_eq!(
            response.0,
            VersionResponse {
                version: "v0.9.2".to_string()
            }
        );
    }
}
