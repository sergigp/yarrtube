use crate::domain::video::{RecentVideo, Video, VideoSource};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct VideoResponse {
    pub id: String,
    pub title: String,
    pub status: String,
    pub quality: Option<String>,
    pub filename: Option<String>,
    pub thumbnail_filename: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Video> for VideoResponse {
    fn from(video: Video) -> Self {
        Self {
            id: video.youtube_id.as_str().to_string(),
            title: video.title,
            status: video.status.as_str().to_string(),
            quality: video.quality.map(|q| q.as_str().to_string()),
            filename: video.filename,
            thumbnail_filename: video.thumbnail_filename,
            created_at: video.created_at,
            updated_at: video.updated_at,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RecentVideoSourceResponse {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RecentVideoResponse {
    pub id: String,
    pub title: String,
    pub source: RecentVideoSourceResponse,
}

impl From<RecentVideo> for RecentVideoResponse {
    fn from(recent_video: RecentVideo) -> Self {
        let source = match recent_video.source {
            VideoSource::Playlist(id) => RecentVideoSourceResponse {
                kind: "playlist".to_string(),
                id: id.as_str().to_string(),
            },
            VideoSource::Channel(id) => RecentVideoSourceResponse {
                kind: "channel".to_string(),
                id: id.as_str().to_string(),
            },
        };
        Self {
            id: recent_video.video.youtube_id.as_str().to_string(),
            title: recent_video.video.title,
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::ChannelHandle;
    use crate::domain::shared::{PlaylistId, Quality, VideoId};
    use crate::domain::video::Video;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn it_should_serialize_a_recorded_thumbnail_filename() {
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "My Video.mp4",
                Some("My Video.jpg".to_string()),
                fixed_timestamp(),
            );

        let response: VideoResponse = video.into();

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["thumbnail_filename"], "My Video.jpg");
    }

    #[test]
    fn it_should_serialize_no_thumbnail_filename_as_null() {
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp());

        let response: VideoResponse = video.into();

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["thumbnail_filename"], serde_json::Value::Null);
    }

    #[test]
    fn it_should_tag_the_source_as_playlist() {
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());

        let response = RecentVideoResponse::from(RecentVideo {
            video,
            source: VideoSource::Playlist(PlaylistId::new("PL1").unwrap()),
        });

        assert_eq!(response.id, "vid1");
        assert_eq!(response.title, "My Video");
        assert_eq!(response.source.kind, "playlist");
        assert_eq!(response.source.id, "PL1");
    }

    #[test]
    fn it_should_tag_the_source_as_channel() {
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());

        let response = RecentVideoResponse::from(RecentVideo {
            video,
            source: VideoSource::Channel(ChannelHandle::new("@somechannel").unwrap()),
        });

        assert_eq!(response.source.kind, "channel");
        assert_eq!(response.source.id, "@somechannel");
    }
}
