use crate::domain::playlist::PlaylistId;
use anyhow::Context;
use serde::Deserialize;

const PLAYLISTS_URL: &str = "https://www.googleapis.com/youtube/v3/playlists";

/// What YouTube reports about a playlist: its title and how many items it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlaylist {
    pub title: String,
    pub item_count: u64,
}

pub trait YoutubePlaylistRepository: Send + Sync {
    fn resolve(&self, id: &PlaylistId) -> anyhow::Result<Option<ResolvedPlaylist>>;
}

#[derive(Debug, Deserialize)]
struct PlaylistsResponse {
    items: Vec<PlaylistItem>,
}

#[derive(Debug, Deserialize)]
struct PlaylistItem {
    snippet: PlaylistSnippet,
    #[serde(rename = "contentDetails")]
    content_details: PlaylistContentDetails,
}

#[derive(Debug, Deserialize)]
struct PlaylistSnippet {
    title: String,
}

#[derive(Debug, Deserialize)]
struct PlaylistContentDetails {
    #[serde(rename = "itemCount")]
    item_count: u64,
}

pub struct YoutubeApiPlaylistRepository {
    api_key: String,
    base_url: String,
}

impl YoutubeApiPlaylistRepository {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: PLAYLISTS_URL.to_string(),
        }
    }

    #[cfg(test)]
    fn with_base_url(api_key: String, base_url: String) -> Self {
        Self { api_key, base_url }
    }
}

impl YoutubePlaylistRepository for YoutubeApiPlaylistRepository {
    fn resolve(&self, id: &PlaylistId) -> anyhow::Result<Option<ResolvedPlaylist>> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&self.base_url)
            .query(&[
                ("part", "snippet,contentDetails"),
                ("id", id.as_str()),
                ("key", self.api_key.as_str()),
            ])
            .send()
            .inspect_err(|_| tracing::error!(playlist_id = %id, "YouTube API request failed"))
            .context("YouTube API request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            tracing::error!(playlist_id = %id, status = %status, "YouTube API request failed");
            anyhow::bail!("YouTube API request failed with status {status}: {body}");
        }

        let parsed: PlaylistsResponse = response
            .json()
            .inspect_err(|e| {
                tracing::error!(playlist_id = %id, error = %e, "failed to parse YouTube API response")
            })
            .context("failed to parse YouTube API response")?;

        Ok(parsed
            .items
            .into_iter()
            .next()
            .map(|item| ResolvedPlaylist {
                title: item.snippet.title,
                item_count: item.content_details.item_count,
            }))
    }
}

#[cfg(test)]
pub struct FakeYoutubePlaylistRepository {
    pub(crate) resolved: Option<ResolvedPlaylist>,
}

#[cfg(test)]
impl YoutubePlaylistRepository for FakeYoutubePlaylistRepository {
    fn resolve(&self, _id: &PlaylistId) -> anyhow::Result<Option<ResolvedPlaylist>> {
        Ok(self.resolved.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_resolve_the_playlist_title_and_item_count() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("id".into(), "PLexists".into()),
                mockito::Matcher::UrlEncoded("part".into(), "snippet,contentDetails".into()),
            ]))
            .with_status(200)
            .with_body(
                r#"{"items": [{
                    "id": "PLexists",
                    "snippet": {"title": "Lofi beats"},
                    "contentDetails": {"itemCount": 42}
                }]}"#,
            )
            .create();

        let repository =
            YoutubeApiPlaylistRepository::with_base_url("api-key".to_string(), server.url());
        let id = PlaylistId::new("PLexists").unwrap();

        assert_eq!(
            repository.resolve(&id).unwrap(),
            Some(ResolvedPlaylist {
                title: "Lofi beats".to_string(),
                item_count: 42,
            })
        );
    }

    #[test]
    fn it_should_resolve_nothing_if_the_playlist_does_not_exist() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("id".into(), "PLmissing".into()),
                mockito::Matcher::UrlEncoded("part".into(), "snippet,contentDetails".into()),
            ]))
            .with_status(200)
            .with_body(r#"{"items": []}"#)
            .create();

        let repository =
            YoutubeApiPlaylistRepository::with_base_url("api-key".to_string(), server.url());
        let id = PlaylistId::new("PLmissing").unwrap();

        assert_eq!(repository.resolve(&id).unwrap(), None);
    }
}
