use crate::domain::video::{
    CategoryVideoView, Download, HomeCategorySection, HomeVideoView, HomeVideos, StepState,
    StepStatus, Video, VideoChapter, VideoSource, VideoStatus, VideoView,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RecordProgressRequest {
    #[serde(default)]
    pub position_seconds: Option<i64>,
    #[serde(default)]
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    pub was_watched: Option<bool>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RecordProgressResponse {
    pub watched: bool,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct VideoResponse {
    pub id: String,
    pub youtube_id: String,
    pub title: String,
    pub status: String,
    pub steps: VideoStepsResponse,
    pub quality: Option<String>,
    pub filename: Option<String>,
    pub thumbnail_filename: Option<String>,
    pub duration_seconds: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub watched: bool,
    pub position_seconds: i64,
    pub synced_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub channel_name: Option<String>,
    pub subtitle_language: Option<String>,
    pub chapters: Vec<ChapterResponse>,
    pub summary: Option<String>,
}

/// Where each of a video's steps stands — see the `video-listing`
/// capability.
#[derive(Debug, Serialize, PartialEq)]
pub struct VideoStepsResponse {
    pub download: StepResponse,
    pub thumbnail: StepResponse,
    pub subtitles: StepResponse,
    pub chapters: StepResponse,
    pub quality: StepResponse,
    pub summary: StepResponse,
    pub categorization: StepResponse,
}

/// One step's status (`PENDING`, `RETRYING`, `DONE`, `UNAVAILABLE`,
/// `FAILED`; the download also `IN_PROGRESS`), its consecutive failed
/// attempts, when its next attempt is due (only while Retrying) and its last
/// failure reason (download only, while Retrying or Failed).
#[derive(Debug, Serialize, PartialEq)]
pub struct StepResponse {
    pub status: String,
    pub failed_attempts: u32,
    pub next_attempt_at: Option<DateTime<Utc>>,
    pub failure: Option<String>,
}

impl From<&Video> for VideoStepsResponse {
    fn from(video: &Video) -> Self {
        Self {
            download: StepResponse::download(&video.download),
            thumbnail: StepResponse::from(video.thumbnail.state),
            subtitles: StepResponse::from(video.subtitles.state),
            chapters: StepResponse::from(video.chapters.state),
            quality: StepResponse::from(video.downloaded_quality.state),
            summary: StepResponse::from(video.summary),
            categorization: StepResponse::from(video.categorization),
        }
    }
}

impl From<StepState> for StepResponse {
    fn from(state: StepState) -> Self {
        Self {
            status: state.status.as_str().to_string(),
            failed_attempts: state.attempts.failed,
            next_attempt_at: state.next_attempt_at(),
            failure: None,
        }
    }
}

impl StepResponse {
    fn download(download: &Download) -> Self {
        let status = match download.status() {
            VideoStatus::Downloaded => StepStatus::Done.as_str(),
            other => other.as_str(),
        };
        let failing = matches!(
            download.status(),
            VideoStatus::Retrying | VideoStatus::Failed
        );
        Self {
            status: status.to_string(),
            failed_attempts: download.state.attempts.failed,
            next_attempt_at: download.next_attempt_at(),
            failure: download.failure.clone().filter(|_| failing),
        }
    }
}

/// One of a video's chapters, in whole seconds from the start of the video.
#[derive(Debug, Serialize, PartialEq)]
pub struct ChapterResponse {
    pub start_seconds: i64,
    pub end_seconds: i64,
    pub title: String,
}

impl From<VideoChapter> for ChapterResponse {
    fn from(chapter: VideoChapter) -> Self {
        Self {
            start_seconds: chapter.start_seconds,
            end_seconds: chapter.end_seconds,
            title: chapter.title,
        }
    }
}

impl From<VideoView> for VideoResponse {
    fn from(view: VideoView) -> Self {
        let video = view.video;
        let file = video.download.file.clone();
        let summary = view
            .summary
            .map(|summary| summary.text.as_str().to_string());
        let (published_at, channel_name) = match view.metadata {
            Some(metadata) => (Some(metadata.published_at), Some(metadata.studio)),
            None => (None, None),
        };
        Self {
            steps: VideoStepsResponse::from(&video),
            subtitle_language: video.subtitles.language(),
            watched: video.watch.is_watched(),
            position_seconds: video.watch.position.seconds(),
            id: video.id.as_str().to_string(),
            youtube_id: video.youtube_id.as_str().to_string(),
            title: video.title,
            status: video.download.status().as_str().to_string(),
            quality: file.as_ref().map(|file| file.quality.as_str().to_string()),
            filename: file.as_ref().map(|file| file.filename.clone()),
            thumbnail_filename: video.thumbnail.filename,
            duration_seconds: file.and_then(|file| file.duration_seconds),
            created_at: video.created_at,
            updated_at: video.updated_at,
            synced_at: video.synced_at,
            published_at,
            channel_name,
            chapters: video
                .chapters
                .found
                .unwrap_or_default()
                .into_iter()
                .map(ChapterResponse::from)
                .collect(),
            summary,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HomeVideoSourceResponse {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub path: String,
    pub avatar_filename: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchVideosQuery {
    #[serde(default)]
    pub q: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SearchVideosResponse {
    pub videos: Vec<HomeVideoResponse>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HomeVideoResponse {
    pub id: String,
    pub youtube_id: String,
    pub title: String,
    pub thumbnail_filename: Option<String>,
    pub duration_seconds: Option<i64>,
    pub watched: bool,
    pub position_seconds: i64,
    pub published_at: Option<DateTime<Utc>>,
    pub source: HomeVideoSourceResponse,
    pub category_ids: Vec<String>,
}

impl From<VideoSource> for HomeVideoSourceResponse {
    fn from(source: VideoSource) -> Self {
        match source {
            VideoSource::Playlist { id, name, path } => Self {
                kind: "playlist".to_string(),
                id: id.as_str().to_string(),
                name: name.as_str().to_string(),
                path: path.as_str().to_string(),
                avatar_filename: None,
            },
            VideoSource::Channel {
                handle,
                name,
                path,
                avatar_filename,
            } => Self {
                kind: "channel".to_string(),
                id: handle.as_str().to_string(),
                name,
                path: path.as_str().to_string(),
                avatar_filename,
            },
        }
    }
}

impl From<HomeVideoView> for HomeVideoResponse {
    fn from(view: HomeVideoView) -> Self {
        let source = HomeVideoSourceResponse::from(view.source);
        Self {
            id: view.id.as_str().to_string(),
            youtube_id: view.youtube_id.as_str().to_string(),
            title: view.title,
            thumbnail_filename: view.thumbnail_filename,
            duration_seconds: view.duration_seconds,
            watched: view.watched,
            position_seconds: view.position_seconds,
            published_at: view.published_at,
            source,
            category_ids: view
                .category_ids
                .iter()
                .map(|id| id.as_str().to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CategoryVideoResponse {
    pub id: String,
    pub youtube_id: String,
    pub title: String,
    pub status: String,
    pub steps: VideoStepsResponse,
    pub quality: Option<String>,
    pub filename: Option<String>,
    pub thumbnail_filename: Option<String>,
    pub duration_seconds: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub watched: bool,
    pub position_seconds: i64,
    pub synced_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub channel_name: Option<String>,
    pub subtitle_language: Option<String>,
    pub chapters: Vec<ChapterResponse>,
    pub summary: Option<String>,
    pub source: HomeVideoSourceResponse,
    pub category_ids: Vec<String>,
}

impl From<CategoryVideoView> for CategoryVideoResponse {
    fn from(view: CategoryVideoView) -> Self {
        // The same fields, mapped the same way, as a source's video listing.
        let video = VideoResponse::from(VideoView {
            video: view.video,
            metadata: view.metadata,
            summary: view.summary,
        });
        Self {
            id: video.id,
            youtube_id: video.youtube_id,
            title: video.title,
            status: video.status,
            steps: video.steps,
            quality: video.quality,
            filename: video.filename,
            thumbnail_filename: video.thumbnail_filename,
            duration_seconds: video.duration_seconds,
            created_at: video.created_at,
            updated_at: video.updated_at,
            watched: video.watched,
            position_seconds: video.position_seconds,
            synced_at: video.synced_at,
            published_at: video.published_at,
            channel_name: video.channel_name,
            subtitle_language: video.subtitle_language,
            chapters: video.chapters,
            summary: video.summary,
            source: HomeVideoSourceResponse::from(view.source),
            category_ids: view
                .category_ids
                .iter()
                .map(|id| id.as_str().to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct HomeResponse {
    pub continue_watching: Vec<HomeVideoResponse>,
    pub categories: Vec<HomeCategorySectionResponse>,
    pub quick_watches: Vec<HomeVideoResponse>,
    pub latest: Vec<HomeVideoResponse>,
}

impl From<HomeVideos> for HomeResponse {
    fn from(home: HomeVideos) -> Self {
        Self {
            continue_watching: cards(home.continue_watching),
            categories: home.categories.into_iter().map(Into::into).collect(),
            quick_watches: cards(home.quick_watches),
            latest: cards(home.latest),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct HomeCategorySectionResponse {
    pub id: String,
    pub name: String,
    pub color: String,
    pub videos: Vec<HomeVideoResponse>,
}

impl From<HomeCategorySection> for HomeCategorySectionResponse {
    fn from(section: HomeCategorySection) -> Self {
        Self {
            id: section.id.as_str().to_string(),
            name: section.name.as_str().to_string(),
            color: section.color.as_str().to_string(),
            videos: cards(section.videos),
        }
    }
}

fn cards(videos: Vec<HomeVideoView>) -> Vec<HomeVideoResponse> {
    videos.into_iter().map(HomeVideoResponse::from).collect()
}
