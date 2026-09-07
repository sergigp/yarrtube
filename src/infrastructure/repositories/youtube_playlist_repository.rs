use crate::domain::playlist::YoutubePlaylistId;
use anyhow::Context;
use serde::Deserialize;

const PLAYLISTS_URL: &str = "https://www.googleapis.com/youtube/v3/playlists";

pub trait YoutubePlaylistRepository: Send + Sync {
    fn exists(&self, id: &YoutubePlaylistId) -> anyhow::Result<bool>;
}

#[derive(Debug, Deserialize)]
struct PlaylistsResponse {
    items: Vec<serde_json::Value>,
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
    fn exists(&self, id: &YoutubePlaylistId) -> anyhow::Result<bool> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&self.base_url)
            .query(&[
                ("part", "id"),
                ("id", id.as_str()),
                ("key", self.api_key.as_str()),
            ])
            .send()
            .context("YouTube API request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("YouTube API request failed with status {status}: {body}");
        }

        let parsed: PlaylistsResponse = response
            .json()
            .context("failed to parse YouTube API response")?;

        Ok(!parsed.items.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_return_true_when_the_playlist_exists() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("id".into(), "PLexists".into()),
                mockito::Matcher::UrlEncoded("part".into(), "id".into()),
            ]))
            .with_status(200)
            .with_body(r#"{"items": [{"id": "PLexists"}]}"#)
            .create();

        let repository =
            YoutubeApiPlaylistRepository::with_base_url("api-key".to_string(), server.url());
        let id = YoutubePlaylistId::new("PLexists").unwrap();

        assert!(repository.exists(&id).unwrap());
    }

    #[test]
    fn it_should_return_false_when_the_playlist_does_not_exist() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("id".into(), "PLmissing".into()),
                mockito::Matcher::UrlEncoded("part".into(), "id".into()),
            ]))
            .with_status(200)
            .with_body(r#"{"items": []}"#)
            .create();

        let repository =
            YoutubeApiPlaylistRepository::with_base_url("api-key".to_string(), server.url());
        let id = YoutubePlaylistId::new("PLmissing").unwrap();

        assert!(!repository.exists(&id).unwrap());
    }
}
