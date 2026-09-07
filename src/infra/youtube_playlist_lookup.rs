use crate::domain::playlist::YoutubePlaylistId;
use crate::domain::ports::{LookupError, YoutubePlaylistLookup};
use serde::Deserialize;

const PLAYLISTS_URL: &str = "https://www.googleapis.com/youtube/v3/playlists";

#[derive(Debug, Deserialize)]
struct PlaylistsResponse {
    items: Vec<serde_json::Value>,
}

pub struct YoutubeApiPlaylistLookup {
    api_key: String,
    base_url: String,
}

impl YoutubeApiPlaylistLookup {
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

impl YoutubePlaylistLookup for YoutubeApiPlaylistLookup {
    fn exists(&self, id: &YoutubePlaylistId) -> Result<bool, LookupError> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&self.base_url)
            .query(&[
                ("part", "id"),
                ("id", id.as_str()),
                ("key", self.api_key.as_str()),
            ])
            .send()
            .map_err(|e| LookupError(format!("YouTube API request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(LookupError(format!(
                "YouTube API request failed with status {status}: {body}"
            )));
        }

        let parsed: PlaylistsResponse = response
            .json()
            .map_err(|e| LookupError(format!("failed to parse YouTube API response: {e}")))?;

        Ok(!parsed.items.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_true_when_playlist_exists() {
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

        let lookup = YoutubeApiPlaylistLookup::with_base_url("api-key".to_string(), server.url());
        let id = YoutubePlaylistId::new("PLexists").unwrap();

        assert!(lookup.exists(&id).unwrap());
    }

    #[test]
    fn returns_false_when_playlist_does_not_exist() {
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

        let lookup = YoutubeApiPlaylistLookup::with_base_url("api-key".to_string(), server.url());
        let id = YoutubePlaylistId::new("PLmissing").unwrap();

        assert!(!lookup.exists(&id).unwrap());
    }
}
