use crate::domain::playlist::PlaylistId;
#[cfg(test)]
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::shared::error_report;
use crate::infrastructure::shared::youtube_api_key::YOUTUBE_API_KEY_HEADER;
use anyhow::{Context, anyhow};
use serde::Deserialize;
use std::collections::HashSet;
#[cfg(test)]
use std::sync::Arc;

const YOUTUBE_API_URL: &str = "https://www.googleapis.com/youtube/v3";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YoutubePlaylistItem {
    pub video_id: String,
    pub title: String,
    pub position: i64,
}

pub trait YoutubePlaylistItemsRepository: Send + Sync {
    fn list_current_videos(
        &self,
        playlist_id: &PlaylistId,
    ) -> anyhow::Result<Vec<YoutubePlaylistItem>>;
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
    status: Option<PlaylistItemStatus>,
}

impl PlaylistItem {
    /// True only for `public` and `unlisted`; private, missing and any
    /// other status (e.g. deleted videos) are not watchable.
    fn is_watchable(&self) -> bool {
        matches!(self.privacy_status(), Some("public" | "unlisted"))
    }

    fn privacy_status(&self) -> Option<&str> {
        self.status
            .as_ref()
            .and_then(|status| status.privacy_status.as_deref())
    }
}

#[derive(Debug, Deserialize)]
struct PlaylistItemSnippet {
    title: String,
    #[serde(rename = "resourceId")]
    resource_id: ResourceId,
    #[serde(rename = "position")]
    position: i64,
}

#[derive(Debug, Deserialize)]
struct PlaylistItemStatus {
    #[serde(rename = "privacyStatus")]
    privacy_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResourceId {
    #[serde(rename = "videoId")]
    video_id: String,
}

#[derive(Debug, Deserialize)]
struct VideosResponse {
    items: Vec<VideoItem>,
}

#[derive(Debug, Deserialize)]
struct VideoItem {
    id: String,
    snippet: VideoSnippet,
}

impl VideoItem {
    /// True for `upcoming` and `live` broadcasts, which can't be downloaded
    /// yet.
    fn is_broadcast_pending(&self) -> bool {
        matches!(
            self.snippet.live_broadcast_content.as_deref(),
            Some("upcoming" | "live")
        )
    }
}

#[derive(Debug, Deserialize)]
struct VideoSnippet {
    #[serde(rename = "liveBroadcastContent")]
    live_broadcast_content: Option<String>,
}

pub struct YoutubeApiPlaylistItemsRepository {
    api_key: String,
    base_url: String,
}

impl YoutubeApiPlaylistItemsRepository {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: YOUTUBE_API_URL.to_string(),
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
    ) -> anyhow::Result<(Vec<YoutubePlaylistItem>, Option<String>)> {
        let client = reqwest::blocking::Client::new();
        let mut query = vec![
            ("part", "snippet,status"),
            ("maxResults", "50"),
            ("playlistId", playlist_id),
        ];
        if let Some(token) = page_token {
            query.push(("pageToken", token));
        }

        let response = client
            .get(format!("{}/playlistItems", self.base_url))
            .query(&query)
            .header(YOUTUBE_API_KEY_HEADER, &self.api_key)
            .send()
            .map_err(|e| anyhow::Error::new(e).context("YouTube API request failed"))
            .inspect_err(|e| {
                tracing::error!(
                    playlist_id,
                    page_token = ?page_token,
                    error = %error_report::cause_chain(e),
                    "YouTube API request failed"
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            let error = anyhow!("YouTube API request failed with status {status}: {body}");
            tracing::error!(
                playlist_id,
                page_token = ?page_token,
                status = %status,
                error = %error_report::cause_chain(&error),
                "YouTube API request failed"
            );
            return Err(error);
        }

        let parsed: PlaylistItemsResponse = response
            .json()
            .inspect_err(|e| {
                tracing::error!(
                    playlist_id,
                    page_token = ?page_token,
                    error = %e,
                    "failed to parse YouTube API response"
                )
            })
            .context("failed to parse YouTube API response")?;
        let items: Vec<PlaylistItem> = parsed
            .items
            .into_iter()
            .filter(PlaylistItem::is_watchable)
            .collect();
        let video_ids: Vec<&str> = items
            .iter()
            .map(|item| item.snippet.resource_id.video_id.as_str())
            .collect();
        let not_yet_watchable = self.find_not_yet_watchable(&video_ids)?;
        let videos = items
            .into_iter()
            .filter(|item| !not_yet_watchable.contains(&item.snippet.resource_id.video_id))
            .map(|item| YoutubePlaylistItem {
                video_id: item.snippet.resource_id.video_id,
                title: item.snippet.title,
                position: item.snippet.position,
            })
            .collect();

        Ok((videos, parsed.next_page_token))
    }

    /// The ids among `video_ids` that YouTube reports as an upcoming or
    /// currently live broadcast.
    fn find_not_yet_watchable(&self, video_ids: &[&str]) -> anyhow::Result<HashSet<String>> {
        let ids = video_ids.join(",");
        let response = reqwest::blocking::Client::new()
            .get(format!("{}/videos", self.base_url))
            .query(&[("part", "snippet"), ("id", ids.as_str())])
            .header(YOUTUBE_API_KEY_HEADER, &self.api_key)
            .send()
            .map_err(|e| anyhow::Error::new(e).context("YouTube API request failed"))
            .inspect_err(|e| {
                tracing::error!(
                    video_ids = %ids,
                    error = %error_report::cause_chain(e),
                    "YouTube API request failed"
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            let error = anyhow!("YouTube API request failed with status {status}: {body}");
            tracing::error!(
                video_ids = %ids,
                status = %status,
                error = %error_report::cause_chain(&error),
                "YouTube API request failed"
            );
            return Err(error);
        }

        let parsed: VideosResponse = response
            .json()
            .inspect_err(|e| {
                tracing::error!(
                    video_ids = %ids,
                    error = %e,
                    "failed to parse YouTube API response"
                )
            })
            .context("failed to parse YouTube API response")?;

        Ok(parsed
            .items
            .into_iter()
            .filter(VideoItem::is_broadcast_pending)
            .map(|item| item.id)
            .collect())
    }
}

impl YoutubePlaylistItemsRepository for YoutubeApiPlaylistItemsRepository {
    fn list_current_videos(
        &self,
        playlist_id: &PlaylistId,
    ) -> anyhow::Result<Vec<YoutubePlaylistItem>> {
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
    pub(crate) videos: std::sync::Mutex<Vec<YoutubePlaylistItem>>,
    fails: bool,
    /// Deletes the listed playlist from this repository, as if a user deleted
    /// it while the reconcile was running.
    deletes_from: Option<Arc<dyn PlaylistRepository>>,
}

#[cfg(test)]
impl FakeYoutubePlaylistItemsRepository {
    pub fn with_videos(videos: Vec<YoutubePlaylistItem>) -> Self {
        Self {
            videos: std::sync::Mutex::new(videos),
            fails: false,
            deletes_from: None,
        }
    }

    pub fn failing() -> Self {
        Self {
            videos: std::sync::Mutex::new(Vec::new()),
            fails: true,
            deletes_from: None,
        }
    }

    pub fn deleting_the_playlist_from(repository: Arc<dyn PlaylistRepository>) -> Self {
        Self {
            deletes_from: Some(repository),
            ..Self::default()
        }
    }
}

#[cfg(test)]
impl YoutubePlaylistItemsRepository for FakeYoutubePlaylistItemsRepository {
    fn list_current_videos(
        &self,
        playlist_id: &PlaylistId,
    ) -> anyhow::Result<Vec<YoutubePlaylistItem>> {
        if let Some(repository) = &self.deletes_from {
            repository.delete(playlist_id)?;
        }
        if self.fails {
            anyhow::bail!("YouTube API failed to list playlist items");
        }
        Ok(self.videos.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing listens on port 1, so the request fails before any response.
    const UNREACHABLE_URL: &str = "http://127.0.0.1:1";

    #[test]
    fn it_should_combine_multiple_pages_in_order() {
        let mut server = mockito::Server::new();
        let _videos = no_broadcasts(&mut server);
        let _page1 = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::UrlEncoded(
                "playlistId".into(),
                "PL1".into(),
            ))
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "One", "resourceId": {"videoId": "1"}, "position": 0}, "status": {"privacyStatus": "public"}},
                    {"snippet": {"title": "Two", "resourceId": {"videoId": "2"}, "position": 1}, "status": {"privacyStatus": "public"}}
                ], "nextPageToken": "page2"}"#,
            )
            .create();
        let _page2 = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::UrlEncoded(
                "pageToken".into(),
                "page2".into(),
            ))
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "Three", "resourceId": {"videoId": "3"}, "position": 2}, "status": {"privacyStatus": "public"}}
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
                YoutubePlaylistItem {
                    video_id: "1".to_string(),
                    title: "One".to_string(),
                    position: 0,
                },
                YoutubePlaylistItem {
                    video_id: "2".to_string(),
                    title: "Two".to_string(),
                    position: 1,
                },
                YoutubePlaylistItem {
                    video_id: "3".to_string(),
                    title: "Three".to_string(),
                    position: 2,
                },
            ]
        );
    }

    #[test]
    fn it_should_request_snippet_and_status_parts() {
        let mut server = mockito::Server::new();
        let _videos = no_broadcasts(&mut server);
        let _mock = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::UrlEncoded(
                "part".into(),
                "snippet,status".into(),
            ))
            .match_header("x-goog-api-key", "api-key")
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "One", "resourceId": {"videoId": "1"}, "position": 0}, "status": {"privacyStatus": "public"}}
                ]}"#,
            )
            .create();

        let repository =
            YoutubeApiPlaylistItemsRepository::with_base_url("api-key".to_string(), server.url());

        let videos = repository
            .list_current_videos(&PlaylistId::new("PL1").unwrap())
            .map_err(|e| e.to_string());

        assert_eq!(
            videos,
            Ok(vec![YoutubePlaylistItem {
                video_id: "1".to_string(),
                title: "One".to_string(),
                position: 0,
            }])
        );
    }

    #[test]
    fn it_should_skip_private_items() {
        let mut server = mockito::Server::new();
        let _videos = no_broadcasts(&mut server);
        let _mock = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "Private video", "resourceId": {"videoId": "1"}, "position": 0}, "status": {"privacyStatus": "private"}},
                    {"snippet": {"title": "Two", "resourceId": {"videoId": "2"}, "position": 1}, "status": {"privacyStatus": "public"}}
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
            vec![YoutubePlaylistItem {
                video_id: "2".to_string(),
                title: "Two".to_string(),
                position: 1,
            }]
        );
    }

    #[test]
    fn it_should_skip_items_without_a_privacy_status() {
        let mut server = mockito::Server::new();
        let _videos = no_broadcasts(&mut server);
        let _mock = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "Deleted video", "resourceId": {"videoId": "1"}, "position": 0}},
                    {"snippet": {"title": "Two", "resourceId": {"videoId": "2"}, "position": 1}, "status": {"privacyStatus": "public"}}
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
            vec![YoutubePlaylistItem {
                video_id: "2".to_string(),
                title: "Two".to_string(),
                position: 1,
            }]
        );
    }

    #[test]
    fn it_should_skip_items_with_an_unrecognised_privacy_status() {
        let mut server = mockito::Server::new();
        let _videos = no_broadcasts(&mut server);
        let _mock = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "One", "resourceId": {"videoId": "1"}, "position": 0}, "status": {"privacyStatus": "privacyStatusUnspecified"}},
                    {"snippet": {"title": "Two", "resourceId": {"videoId": "2"}, "position": 1}, "status": {"privacyStatus": "public"}}
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
            vec![YoutubePlaylistItem {
                video_id: "2".to_string(),
                title: "Two".to_string(),
                position: 1,
            }]
        );
    }

    #[test]
    fn it_should_keep_unlisted_items() {
        let mut server = mockito::Server::new();
        let _videos = no_broadcasts(&mut server);
        let _mock = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "Unlisted", "resourceId": {"videoId": "1"}, "position": 3}, "status": {"privacyStatus": "unlisted"}}
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
            vec![YoutubePlaylistItem {
                video_id: "1".to_string(),
                title: "Unlisted".to_string(),
                position: 3,
            }]
        );
    }

    #[test]
    fn it_should_skip_items_reported_as_upcoming() {
        let mut server = mockito::Server::new();
        let _items = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "Premiere", "resourceId": {"videoId": "1"}, "position": 0}, "status": {"privacyStatus": "public"}},
                    {"snippet": {"title": "Two", "resourceId": {"videoId": "2"}, "position": 1}, "status": {"privacyStatus": "public"}}
                ]}"#,
            )
            .create();
        let _videos = server
            .mock("GET", "/videos")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"id": "1", "snippet": {"liveBroadcastContent": "upcoming"}},
                    {"id": "2", "snippet": {"liveBroadcastContent": "none"}}
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
            vec![YoutubePlaylistItem {
                video_id: "2".to_string(),
                title: "Two".to_string(),
                position: 1,
            }]
        );
    }

    #[test]
    fn it_should_skip_items_reported_as_live() {
        let mut server = mockito::Server::new();
        let _items = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "Live now", "resourceId": {"videoId": "1"}, "position": 0}, "status": {"privacyStatus": "public"}},
                    {"snippet": {"title": "Two", "resourceId": {"videoId": "2"}, "position": 1}, "status": {"privacyStatus": "public"}}
                ]}"#,
            )
            .create();
        let _videos = server
            .mock("GET", "/videos")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"id": "1", "snippet": {"liveBroadcastContent": "live"}},
                    {"id": "2", "snippet": {"liveBroadcastContent": "none"}}
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
            vec![YoutubePlaylistItem {
                video_id: "2".to_string(),
                title: "Two".to_string(),
                position: 1,
            }]
        );
    }

    #[test]
    fn it_should_request_live_status_for_the_page_video_ids() {
        let mut server = mockito::Server::new();
        let _items = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "One", "resourceId": {"videoId": "1"}, "position": 0}, "status": {"privacyStatus": "public"}},
                    {"snippet": {"title": "Two", "resourceId": {"videoId": "2"}, "position": 1}, "status": {"privacyStatus": "public"}}
                ]}"#,
            )
            .create();
        let _videos = server
            .mock("GET", "/videos")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("part".into(), "snippet".into()),
                mockito::Matcher::UrlEncoded("id".into(), "1,2".into()),
            ]))
            .match_header("x-goog-api-key", "api-key")
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"id": "1", "snippet": {"liveBroadcastContent": "upcoming"}}
                ]}"#,
            )
            .create();

        let repository =
            YoutubeApiPlaylistItemsRepository::with_base_url("api-key".to_string(), server.url());

        let videos = repository
            .list_current_videos(&PlaylistId::new("PL1").unwrap())
            .map_err(|e| e.to_string());

        assert_eq!(
            videos,
            Ok(vec![YoutubePlaylistItem {
                video_id: "2".to_string(),
                title: "Two".to_string(),
                position: 1,
            }])
        );
    }

    #[test]
    fn it_should_keep_items_missing_from_the_videos_response() {
        let mut server = mockito::Server::new();
        let _items = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "One", "resourceId": {"videoId": "1"}, "position": 0}, "status": {"privacyStatus": "public"}},
                    {"snippet": {"title": "Two", "resourceId": {"videoId": "2"}, "position": 1}, "status": {"privacyStatus": "public"}}
                ]}"#,
            )
            .create();
        let _videos = server
            .mock("GET", "/videos")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"id": "2", "snippet": {"liveBroadcastContent": "none"}}
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
                YoutubePlaylistItem {
                    video_id: "1".to_string(),
                    title: "One".to_string(),
                    position: 0,
                },
                YoutubePlaylistItem {
                    video_id: "2".to_string(),
                    title: "Two".to_string(),
                    position: 1,
                },
            ]
        );
    }

    #[test]
    fn it_should_fail_when_the_live_status_lookup_fails() {
        let mut server = mockito::Server::new();
        let _items = server
            .mock("GET", "/playlistItems")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [
                    {"snippet": {"title": "One", "resourceId": {"videoId": "1"}, "position": 0}, "status": {"privacyStatus": "public"}}
                ]}"#,
            )
            .create();
        let _videos = server
            .mock("GET", "/videos")
            .match_query(mockito::Matcher::Any)
            .with_status(500)
            .with_body("backend error")
            .create();

        let repository =
            YoutubeApiPlaylistItemsRepository::with_base_url("api-key".to_string(), server.url());

        let result = repository
            .list_current_videos(&PlaylistId::new("PL1").unwrap())
            .map_err(|e| e.to_string());

        assert_eq!(
            result,
            Err(
                "YouTube API request failed with status 500 Internal Server Error: backend error"
                    .to_string()
            )
        );
    }

    #[test]
    fn it_should_return_an_empty_list_when_the_playlist_has_no_videos() {
        let mut server = mockito::Server::new();
        let _videos = no_broadcasts(&mut server);
        let _mock = server
            .mock("GET", "/playlistItems")
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

    #[test]
    fn it_should_keep_the_api_key_out_of_a_connection_error() {
        let repository = YoutubeApiPlaylistItemsRepository::with_base_url(
            "secret-api-key".to_string(),
            UNREACHABLE_URL.to_string(),
        );

        let error = repository
            .list_current_videos(&PlaylistId::new("PL1").unwrap())
            .unwrap_err();

        assert!(
            !format!("{error:?}").contains("secret-api-key"),
            "the API key leaked into: {error:?}"
        );
    }

    /// A `/videos` response reporting none of the page's items as an
    /// upcoming or live broadcast.
    fn no_broadcasts(server: &mut mockito::Server) -> mockito::Mock {
        server
            .mock("GET", "/videos")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(r#"{"items": []}"#)
            .create()
    }
}
