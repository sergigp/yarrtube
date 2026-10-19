use crate::infrastructure::shared::app_version::{AppVersion, BuildChannel, BuildCommit};
use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub channel: ChannelResponse,
    pub commit: Option<CommitResponse>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelResponse {
    Release,
    Edge,
    Dev,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct CommitResponse {
    pub sha: String,
    pub subject: String,
    pub date: String,
}

impl From<AppVersion> for VersionResponse {
    fn from(app_version: AppVersion) -> Self {
        Self {
            version: app_version.version,
            channel: app_version.channel.into(),
            commit: app_version.commit.map(CommitResponse::from),
        }
    }
}

impl From<BuildChannel> for ChannelResponse {
    fn from(channel: BuildChannel) -> Self {
        match channel {
            BuildChannel::Release => Self::Release,
            BuildChannel::Edge => Self::Edge,
            BuildChannel::Dev => Self::Dev,
        }
    }
}

impl From<BuildCommit> for CommitResponse {
    fn from(commit: BuildCommit) -> Self {
        Self {
            sha: commit.sha,
            subject: commit.subject,
            date: commit.date,
        }
    }
}
