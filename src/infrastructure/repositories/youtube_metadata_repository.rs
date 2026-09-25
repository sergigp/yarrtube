use crate::domain::video::VideoId;
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Deserialize;

const VIDEOS_URL: &str = "https://www.googleapis.com/youtube/v3/videos";

/// A video's full YouTube snippet, fetched for `movie.nfo` generation.
#[derive(Debug, Clone, PartialEq)]
pub struct YoutubeMetadata {
    pub title: String,
    pub description: String,
    pub channel_title: String,
    pub published_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub category_id: Option<String>,
}

pub trait YoutubeMetadataRepository: Send + Sync {
    fn find(&self, id: &VideoId) -> anyhow::Result<Option<YoutubeMetadata>>;
}

#[derive(Debug, Deserialize)]
struct VideosResponse {
    items: Vec<VideoItem>,
}

#[derive(Debug, Deserialize)]
struct VideoItem {
    snippet: VideoItemSnippet,
}

#[derive(Debug, Deserialize)]
struct VideoItemSnippet {
    title: String,
    description: String,
    #[serde(rename = "channelTitle")]
    channel_title: String,
    #[serde(rename = "publishedAt")]
    published_at: DateTime<Utc>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(rename = "categoryId", default)]
    category_id: Option<String>,
}

pub struct YoutubeApiMetadataRepository {
    api_key: String,
    base_url: String,
}

impl YoutubeApiMetadataRepository {
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

impl YoutubeMetadataRepository for YoutubeApiMetadataRepository {
    fn find(&self, id: &VideoId) -> anyhow::Result<Option<YoutubeMetadata>> {
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&self.base_url)
            .query(&[
                ("part", "snippet"),
                ("id", id.as_str()),
                ("key", self.api_key.as_str()),
            ])
            .send()
            .inspect_err(|_| tracing::error!(video_id = %id, "YouTube API request failed"))
            .context("YouTube API request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            tracing::error!(video_id = %id, status = %status, "YouTube API request failed");
            anyhow::bail!("YouTube API request failed with status {status}: {body}");
        }

        let parsed: VideosResponse = response
            .json()
            .inspect_err(|e| {
                tracing::error!(video_id = %id, error = %e, "failed to parse YouTube API response")
            })
            .context("failed to parse YouTube API response")?;

        Ok(parsed.items.into_iter().next().map(|item| YoutubeMetadata {
            title: item.snippet.title,
            description: item.snippet.description,
            channel_title: item.snippet.channel_title,
            published_at: item.snippet.published_at,
            tags: item.snippet.tags,
            category_id: item.snippet.category_id,
        }))
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeYoutubeMetadataRepository {
    pub(crate) metadata: Option<YoutubeMetadata>,
}

#[cfg(test)]
impl YoutubeMetadataRepository for FakeYoutubeMetadataRepository {
    fn find(&self, _id: &VideoId) -> anyhow::Result<Option<YoutubeMetadata>> {
        Ok(self.metadata.clone())
    }
}

/// A hardcoded YouTube `categoryId` → genre-name mapping — see design.md's
/// "Genre mapping: hardcoded table" decision. An unmapped or missing
/// category ID yields `None` rather than failing metadata generation.
pub fn map_category_to_genre(category_id: Option<&str>) -> Option<String> {
    let name = match category_id? {
        "1" => "Film & Animation",
        "2" => "Autos & Vehicles",
        "10" => "Music",
        "15" => "Pets & Animals",
        "17" => "Sports",
        "19" => "Travel & Events",
        "20" => "Gaming",
        "22" => "People & Blogs",
        "23" => "Comedy",
        "24" => "Entertainment",
        "25" => "News & Politics",
        "26" => "Howto & Style",
        "27" => "Education",
        "28" => "Science & Technology",
        _ => return None,
    };
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn published_at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2024-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn it_should_return_the_metadata_when_the_video_exists() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("id".into(), "vid1".into()),
                mockito::Matcher::UrlEncoded("part".into(), "snippet".into()),
            ]))
            .with_status(200)
            .with_body(
                r#"{"items": [{"id": "vid1", "snippet": {
                    "title": "My Video",
                    "description": "A description",
                    "channelTitle": "My Channel",
                    "publishedAt": "2024-01-02T03:04:05Z",
                    "tags": ["tag1", "tag2"],
                    "categoryId": "10"
                }}]}"#,
            )
            .create();

        let repository =
            YoutubeApiMetadataRepository::with_base_url("api-key".to_string(), server.url());
        let id = VideoId::new("vid1").unwrap();

        let found = repository.find(&id).unwrap().unwrap();
        assert_eq!(found.title, "My Video");
        assert_eq!(found.description, "A description");
        assert_eq!(found.channel_title, "My Channel");
        assert_eq!(found.published_at, published_at());
        assert_eq!(found.tags, vec!["tag1".to_string(), "tag2".to_string()]);
        assert_eq!(found.category_id, Some("10".to_string()));
    }

    #[test]
    fn it_should_default_tags_and_category_id_when_absent() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_body(
                r#"{"items": [{"id": "vid1", "snippet": {
                    "title": "My Video",
                    "description": "A description",
                    "channelTitle": "My Channel",
                    "publishedAt": "2024-01-02T03:04:05Z"
                }}]}"#,
            )
            .create();

        let repository =
            YoutubeApiMetadataRepository::with_base_url("api-key".to_string(), server.url());
        let id = VideoId::new("vid1").unwrap();

        let found = repository.find(&id).unwrap().unwrap();
        assert!(found.tags.is_empty());
        assert_eq!(found.category_id, None);
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
            YoutubeApiMetadataRepository::with_base_url("api-key".to_string(), server.url());
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
            YoutubeApiMetadataRepository::with_base_url("api-key".to_string(), server.url());
        let id = VideoId::new("vid1").unwrap();

        assert!(repository.find(&id).is_err());
    }

    #[test]
    fn it_should_map_a_known_category_id_to_its_genre_name() {
        assert_eq!(map_category_to_genre(Some("10")), Some("Music".to_string()));
    }

    #[test]
    fn it_should_return_none_for_an_unmapped_category_id() {
        assert_eq!(map_category_to_genre(Some("999999")), None);
    }

    #[test]
    fn it_should_return_none_for_a_missing_category_id() {
        assert_eq!(map_category_to_genre(None), None);
    }
}
