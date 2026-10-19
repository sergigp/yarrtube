/// The build this binary was made from, stamped by the image build:
/// `YARRTUBE_VERSION` (the release tag, or `edge-<short sha>`) and
/// `YARRTUBE_COMMIT_{SHA,SUBJECT,DATE}`. A build without them (e.g. a local
/// one) reports `dev` and no commit. Cargo.toml's version is deliberately not
/// used: releases are cut by tag only.
#[derive(Clone, Debug, PartialEq)]
pub struct AppVersion {
    pub version: String,
    pub channel: BuildChannel,
    pub commit: Option<BuildCommit>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BuildChannel {
    Release,
    Edge,
    Dev,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuildCommit {
    pub sha: String,
    pub subject: String,
    pub date: String,
}

impl AppVersion {
    pub fn new(
        version: Option<&str>,
        commit_sha: Option<&str>,
        commit_subject: Option<&str>,
        commit_date: Option<&str>,
    ) -> Self {
        match present(version) {
            Some(version) => Self {
                version: version.to_string(),
                channel: channel_of(version),
                commit: commit_of(commit_sha, commit_subject, commit_date),
            },
            None => Self {
                version: "dev".to_string(),
                channel: BuildChannel::Dev,
                commit: None,
            },
        }
    }

    /// The version stamped into this binary at compile time.
    pub fn current() -> Self {
        Self::new(
            option_env!("YARRTUBE_VERSION"),
            option_env!("YARRTUBE_COMMIT_SHA"),
            option_env!("YARRTUBE_COMMIT_SUBJECT"),
            option_env!("YARRTUBE_COMMIT_DATE"),
        )
    }
}

fn channel_of(version: &str) -> BuildChannel {
    if version.starts_with("edge-") {
        BuildChannel::Edge
    } else {
        BuildChannel::Release
    }
}

/// The commit, only when all of its fields were stamped.
fn commit_of(sha: Option<&str>, subject: Option<&str>, date: Option<&str>) -> Option<BuildCommit> {
    match (present(sha), present(subject), present(date)) {
        (Some(sha), Some(subject), Some(date)) => Some(BuildCommit {
            sha: sha.to_string(),
            subject: subject.to_string(),
            date: date.to_string(),
        }),
        _ => None,
    }
}

/// An unset build arg and an empty one mean the same thing.
fn present(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_report_dev_without_a_build_version() {
        assert_eq!(
            AppVersion::current(),
            AppVersion {
                version: "dev".to_string(),
                channel: BuildChannel::Dev,
                commit: None,
            }
        );
    }

    #[test]
    fn it_should_report_a_release_build() {
        let version = AppVersion::new(
            Some("v0.9.5"),
            Some("abc1234def5678"),
            Some("Fix sync"),
            Some("2026-10-19T10:00:00Z"),
        );

        assert_eq!(
            version,
            AppVersion {
                version: "v0.9.5".to_string(),
                channel: BuildChannel::Release,
                commit: Some(a_commit()),
            }
        );
    }

    #[test]
    fn it_should_report_an_edge_build() {
        let version = AppVersion::new(
            Some("edge-abc1234"),
            Some("abc1234def5678"),
            Some("Fix sync"),
            Some("2026-10-19T10:00:00Z"),
        );

        assert_eq!(
            version,
            AppVersion {
                version: "edge-abc1234".to_string(),
                channel: BuildChannel::Edge,
                commit: Some(a_commit()),
            }
        );
    }

    #[test]
    fn it_should_report_dev_if_version_is_empty() {
        let version = AppVersion::new(Some(""), None, None, None);

        assert_eq!(
            version,
            AppVersion {
                version: "dev".to_string(),
                channel: BuildChannel::Dev,
                commit: None,
            }
        );
    }

    #[test]
    fn it_should_report_no_commit_if_any_commit_field_is_missing() {
        let version = AppVersion::new(
            Some("edge-abc1234"),
            Some("abc1234def5678"),
            Some(""),
            Some("2026-10-19T10:00:00Z"),
        );

        assert_eq!(
            version,
            AppVersion {
                version: "edge-abc1234".to_string(),
                channel: BuildChannel::Edge,
                commit: None,
            }
        );
    }

    fn a_commit() -> BuildCommit {
        BuildCommit {
            sha: "abc1234def5678".to_string(),
            subject: "Fix sync".to_string(),
            date: "2026-10-19T10:00:00Z".to_string(),
        }
    }
}
