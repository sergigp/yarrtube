use crate::domain::channel::ChannelHandle;
use anyhow::Context;
use serde::Deserialize;

const CHANNELS_URL: &str = "https://www.googleapis.com/youtube/v3/channels";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedChannel {
    pub youtube_channel_id: String,
    pub title: String,
}

pub trait YoutubeChannelRepository: Send + Sync {
    fn resolve(&self, handle: &ChannelHandle) -> anyhow::Result<Option<ResolvedChannel>>;
}

#[derive(Debug, Deserialize)]
struct ChannelsResponse {
    items: Vec<ChannelItem>,
}

#[derive(Debug, Deserialize)]
struct ChannelItem {
    id: String,
    snippet: ChannelSnippet,
}

#[derive(Debug, Deserialize)]
struct ChannelSnippet {
    title: String,
}

pub struct YoutubeApiChannelRepository {
    api_key: String,
    base_url: String,
}

impl YoutubeApiChannelRepository {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: CHANNELS_URL.to_string(),
        }
    }

    #[cfg(test)]
    fn with_base_url(api_key: String, base_url: String) -> Self {
        Self { api_key, base_url }
    }
}

impl YoutubeChannelRepository for YoutubeApiChannelRepository {
    fn resolve(&self, handle: &ChannelHandle) -> anyhow::Result<Option<ResolvedChannel>> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&self.base_url)
            .query(&[
                ("part", "id,snippet"),
                ("forHandle", handle.as_str()),
                ("key", self.api_key.as_str()),
            ])
            .send()
            .inspect_err(|_| tracing::error!(handle = %handle, "YouTube API request failed"))
            .context("YouTube API request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            tracing::error!(handle = %handle, status = %status, "YouTube API request failed");
            anyhow::bail!("YouTube API request failed with status {status}: {body}");
        }

        let parsed: ChannelsResponse = response
            .json()
            .inspect_err(|e| {
                tracing::error!(handle = %handle, error = %e, "failed to parse YouTube API response")
            })
            .context("failed to parse YouTube API response")?;

        Ok(parsed.items.into_iter().next().map(|item| ResolvedChannel {
            youtube_channel_id: item.id,
            title: item.snippet.title,
        }))
    }
}

#[cfg(test)]
pub struct FakeYoutubeChannelRepository {
    pub(crate) resolved: Option<ResolvedChannel>,
}

#[cfg(test)]
impl YoutubeChannelRepository for FakeYoutubeChannelRepository {
    fn resolve(&self, _handle: &ChannelHandle) -> anyhow::Result<Option<ResolvedChannel>> {
        Ok(self.resolved.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_return_the_resolved_channel_when_the_handle_exists() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("forHandle".into(), "@somechannel".into()),
                mockito::Matcher::UrlEncoded("part".into(), "id,snippet".into()),
            ]))
            .with_status(200)
            .with_body(r#"{"items": [{"id": "UC123", "snippet": {"title": "Some Channel"}}]}"#)
            .create();

        let repository =
            YoutubeApiChannelRepository::with_base_url("api-key".to_string(), server.url());
        let handle = ChannelHandle::new("@somechannel").unwrap();

        let resolved = repository.resolve(&handle).unwrap().unwrap();
        assert_eq!(resolved.youtube_channel_id, "UC123");
        assert_eq!(resolved.title, "Some Channel");
    }

    #[test]
    fn it_should_return_none_when_the_handle_does_not_resolve_to_a_channel() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("forHandle".into(), "@missing".into()),
                mockito::Matcher::UrlEncoded("part".into(), "id,snippet".into()),
            ]))
            .with_status(200)
            .with_body(r#"{"items": []}"#)
            .create();

        let repository =
            YoutubeApiChannelRepository::with_base_url("api-key".to_string(), server.url());
        let handle = ChannelHandle::new("@missing").unwrap();

        assert!(repository.resolve(&handle).unwrap().is_none());
    }
}
