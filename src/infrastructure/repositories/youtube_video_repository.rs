use crate::domain::shared::VideoId;
use anyhow::Context;
use serde::Deserialize;

const VIDEOS_URL: &str = "https://www.googleapis.com/youtube/v3/videos";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YoutubeVideo {
    pub video_id: String,
    pub title: String,
}

/// Confirms a single video's existence/accessibility and fetches its title,
/// injected into `VideoService` for adding a video to a custom playlist.
/// Sibling to `YoutubePlaylistRepository`/`YoutubePlaylistItemsRepository`.
pub trait YoutubeVideoRepository: Send + Sync {
    fn find(&self, id: &VideoId) -> anyhow::Result<Option<YoutubeVideo>>;
}

#[derive(Debug, Deserialize)]
struct VideosResponse {
    items: Vec<VideoItem>,
}

#[derive(Debug, Deserialize)]
struct VideoItem {
    id: String,
    snippet: VideoItemSnippet,
}

#[derive(Debug, Deserialize)]
struct VideoItemSnippet {
    title: String,
}

pub struct YoutubeApiVideoRepository {
    api_key: String,
    base_url: String,
}

impl YoutubeApiVideoRepository {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: VIDEOS_URL.to_string(),
        }
    }

    #[cfg(test)]
    fn with_base_url(api_key: String, base_url: String) -> Self {
        Self { api_key, base_url }
    }
}

impl YoutubeVideoRepository for YoutubeApiVideoRepository {
    fn find(&self, id: &VideoId) -> anyhow::Result<Option<YoutubeVideo>> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&self.base_url)
            .query(&[
                ("part", "snippet"),
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

        let parsed: VideosResponse = response
            .json()
            .context("failed to parse YouTube API response")?;

        Ok(parsed.items.into_iter().next().map(|item| YoutubeVideo {
            video_id: item.id,
            title: item.snippet.title,
        }))
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeYoutubeVideoRepository {
    pub(crate) video: Option<YoutubeVideo>,
}

#[cfg(test)]
impl YoutubeVideoRepository for FakeYoutubeVideoRepository {
    fn find(&self, _id: &VideoId) -> anyhow::Result<Option<YoutubeVideo>> {
        Ok(self.video.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_return_the_video_when_it_exists() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("id".into(), "vid1".into()),
                mockito::Matcher::UrlEncoded("part".into(), "snippet".into()),
            ]))
            .with_status(200)
            .with_body(r#"{"items": [{"id": "vid1", "snippet": {"title": "My Video"}}]}"#)
            .create();

        let repository =
            YoutubeApiVideoRepository::with_base_url("api-key".to_string(), server.url());
        let id = VideoId::new("vid1").unwrap();

        let found = repository.find(&id).unwrap().unwrap();
        assert_eq!(found.video_id, "vid1");
        assert_eq!(found.title, "My Video");
    }

    #[test]
    fn it_should_return_none_when_the_video_does_not_exist() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("id".into(), "missing".into()),
                mockito::Matcher::UrlEncoded("part".into(), "snippet".into()),
            ]))
            .with_status(200)
            .with_body(r#"{"items": []}"#)
            .create();

        let repository =
            YoutubeApiVideoRepository::with_base_url("api-key".to_string(), server.url());
        let id = VideoId::new("missing").unwrap();

        assert!(repository.find(&id).unwrap().is_none());
    }

    #[test]
    fn it_should_return_an_error_when_the_api_request_fails() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::Any)
            .with_status(500)
            .with_body("internal error")
            .create();

        let repository =
            YoutubeApiVideoRepository::with_base_url("api-key".to_string(), server.url());
        let id = VideoId::new("vid1").unwrap();

        assert!(repository.find(&id).is_err());
    }
}
