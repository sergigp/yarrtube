use crate::domain::shared::PlaylistId;
use anyhow::{Context, anyhow};
use serde::Deserialize;

const PLAYLIST_ITEMS_URL: &str = "https://www.googleapis.com/youtube/v3/playlistItems";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistVideo {
    pub video_id: String,
    pub title: String,
}

pub trait YoutubePlaylistItemsRepository: Send + Sync {
    fn list_current_videos(&self, playlist_id: &PlaylistId) -> anyhow::Result<Vec<PlaylistVideo>>;
}

#[derive(Debug, Deserialize)]
struct PlaylistItemsResponse {
    items: Vec<PlaylistItem>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlaylistItem {
    snippet: PlaylistItemSnippet,
}

#[derive(Debug, Deserialize)]
struct PlaylistItemSnippet {
    title: String,
    #[serde(rename = "resourceId")]
    resource_id: ResourceId,
}

#[derive(Debug, Deserialize)]
struct ResourceId {
    #[serde(rename = "videoId")]
    video_id: String,
}

pub struct YoutubeApiPlaylistItemsRepository {
    api_key: String,
    base_url: String,
}

impl YoutubeApiPlaylistItemsRepository {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: PLAYLIST_ITEMS_URL.to_string(),
        }
    }

    #[cfg(test)]
    fn with_base_url(api_key: String, base_url: String) -> Self {
        Self { api_key, base_url }
    }

    fn fetch_page(
        &self,
        playlist_id: &str,
        page_token: Option<&str>,
    ) -> anyhow::Result<(Vec<PlaylistVideo>, Option<String>)> {
        let client = reqwest::blocking::Client::new();
        let mut query = vec![
            ("part", "snippet"),
            ("maxResults", "50"),
            ("playlistId", playlist_id),
            ("key", self.api_key.as_str()),
        ];
        if let Some(token) = page_token {
            query.push(("pageToken", token));
        }

        let response = client
            .get(&self.base_url)
            .query(&query)
            .send()
            .context("YouTube API request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!(
                "YouTube API request failed with status {status}: {body}"
            ));
        }

        let parsed: PlaylistItemsResponse = response
            .json()
            .context("failed to parse YouTube API response")?;
        let videos = parsed
            .items
            .into_iter()
            .map(|item| PlaylistVideo {
                video_id: item.snippet.resource_id.video_id,
                title: item.snippet.title,
            })
            .collect();

        Ok((videos, parsed.next_page_token))
    }
}

impl YoutubePlaylistItemsRepository for YoutubeApiPlaylistItemsRepository {
    fn list_current_videos(&self, playlist_id: &PlaylistId) -> anyhow::Result<Vec<PlaylistVideo>> {
        let mut all_videos = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let (mut videos, next_page_token) =
                self.fetch_page(playlist_id.as_str(), page_token.as_deref())?;
            all_videos.append(&mut videos);

            match next_page_token {
                Some(token) => page_token = Some(token),
                None => break,
            }
        }

        Ok(all_videos)
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeYoutubePlaylistItemsRepository {
    pub(crate) videos: Vec<PlaylistVideo>,
}

#[cfg(test)]
impl YoutubePlaylistItemsRepository for FakeYoutubePlaylistItemsRepository {
    fn list_current_videos(&self, _playlist_id: &PlaylistId) -> anyhow::Result<Vec<PlaylistVideo>> {
        Ok(self.videos.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_combine_multiple_pages_in_order() {
        let mut server = mockito::Server::new();
        let _page1 = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::UrlEncoded(
                "playlistId".into(),
                "PL1".into(),
            ))
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "One", "resourceId": {"videoId": "1"}}},
                    {"snippet": {"title": "Two", "resourceId": {"videoId": "2"}}}
                ], "nextPageToken": "page2"}"#,
            )
            .create();
        let _page2 = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::UrlEncoded(
                "pageToken".into(),
                "page2".into(),
            ))
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "Three", "resourceId": {"videoId": "3"}}}
                ]}"#,
            )
            .create();

        let repository =
            YoutubeApiPlaylistItemsRepository::with_base_url("api-key".to_string(), server.url());

        let videos = repository
            .list_current_videos(&PlaylistId::new("PL1").unwrap())
            .unwrap();

        assert_eq!(
            videos,
            vec![
                PlaylistVideo {
                    video_id: "1".to_string(),
                    title: "One".to_string(),
                },
                PlaylistVideo {
                    video_id: "2".to_string(),
                    title: "Two".to_string(),
                },
                PlaylistVideo {
                    video_id: "3".to_string(),
                    title: "Three".to_string(),
                },
            ]
        );
    }

    #[test]
    fn it_should_return_an_empty_list_when_the_playlist_has_no_videos() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(r#"{"items": []}"#)
            .create();

        let repository =
            YoutubeApiPlaylistItemsRepository::with_base_url("api-key".to_string(), server.url());

        let videos = repository
            .list_current_videos(&PlaylistId::new("PL1").unwrap())
            .unwrap();

        assert!(videos.is_empty());
    }
}
