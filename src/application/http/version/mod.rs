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
mod tests {
    use super::*;
    use crate::infrastructure::shared::app_version::AppVersion;
    use dto::{ChannelResponse, CommitResponse};

    #[tokio::test]
    async fn it_should_return_the_running_version() {
        let running_version = RunningVersion(AppVersion::new(
            Some("edge-abc1234"),
            Some("abc1234def5678"),
            Some("Fix sync"),
            Some("2026-10-19T10:00:00Z"),
        ));

        let response = get_version(State(running_version)).await;

        assert_eq!(
            response.0,
            VersionResponse {
                version: "edge-abc1234".to_string(),
                channel: ChannelResponse::Edge,
                commit: Some(CommitResponse {
                    sha: "abc1234def5678".to_string(),
                    subject: "Fix sync".to_string(),
                    date: "2026-10-19T10:00:00Z".to_string(),
                }),
            }
        );
    }

    #[tokio::test]
    async fn it_should_return_dev_for_a_local_build() {
        let running_version = RunningVersion(AppVersion::new(None, None, None, None));

        let response = get_version(State(running_version)).await;

        assert_eq!(
            response.0,
            VersionResponse {
                version: "dev".to_string(),
                channel: ChannelResponse::Dev,
                commit: None,
            }
        );
    }
}
