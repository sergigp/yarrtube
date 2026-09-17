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
    pub duration_seconds: Option<i64>,
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
            duration_seconds: video.duration_seconds,
            created_at: video.created_at,
            updated_at: video.updated_at,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RecentVideoSourceResponse {
    pub kind: String,
    pub id: String,
    pub path: String,
    pub avatar_filename: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RecentVideoResponse {
    pub id: String,
    pub title: String,
    pub thumbnail_filename: Option<String>,
    pub duration_seconds: Option<i64>,
    pub source: RecentVideoSourceResponse,
}

impl From<RecentVideo> for RecentVideoResponse {
    fn from(recent_video: RecentVideo) -> Self {
        let source = match recent_video.source {
            VideoSource::Playlist(id, path) => RecentVideoSourceResponse {
                kind: "playlist".to_string(),
                id: id.as_str().to_string(),
                path: path.as_str().to_string(),
                avatar_filename: None,
            },
            VideoSource::Channel(id, path, avatar_filename) => RecentVideoSourceResponse {
                kind: "channel".to_string(),
                id: id.as_str().to_string(),
                path: path.as_str().to_string(),
                avatar_filename,
            },
        };
        Self {
            id: recent_video.video.youtube_id.as_str().to_string(),
            title: recent_video.video.title,
            thumbnail_filename: recent_video.video.thumbnail_filename,
            duration_seconds: recent_video.video.duration_seconds,
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::ChannelHandle;
    use crate::domain::playlist::PlaylistPath;
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
                None,
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
    fn it_should_serialize_a_recorded_duration() {
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "My Video.mp4",
                None,
                Some(223),
                fixed_timestamp(),
            );

        let response: VideoResponse = video.into();

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["duration_seconds"], 223);
    }

    #[test]
    fn it_should_serialize_no_duration_as_null() {
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp());

        let response: VideoResponse = video.into();

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["duration_seconds"], serde_json::Value::Null);
    }

    #[test]
    fn it_should_tag_the_source_as_playlist() {
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());

        let response = RecentVideoResponse::from(RecentVideo {
            video,
            source: VideoSource::Playlist(
                PlaylistId::new("PL1").unwrap(),
                PlaylistPath::new("music").unwrap(),
            ),
        });

        assert_eq!(response.id, "vid1");
        assert_eq!(response.title, "My Video");
        assert_eq!(response.source.kind, "playlist");
        assert_eq!(response.source.id, "PL1");
        assert_eq!(response.source.path, "music");
    }

    #[test]
    fn it_should_tag_the_source_as_channel() {
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());

        let response = RecentVideoResponse::from(RecentVideo {
            video,
            source: VideoSource::Channel(
                ChannelHandle::new("@somechannel").unwrap(),
                PlaylistPath::new("creators/somechannel").unwrap(),
                None,
            ),
        });

        assert_eq!(response.source.kind, "channel");
        assert_eq!(response.source.id, "@somechannel");
        assert_eq!(response.source.path, "creators/somechannel");
    }

    #[test]
    fn it_should_include_the_channel_avatar_filename_when_present() {
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());

        let response = RecentVideoResponse::from(RecentVideo {
            video,
            source: VideoSource::Channel(
                ChannelHandle::new("@somechannel").unwrap(),
                PlaylistPath::new("creators/somechannel").unwrap(),
                Some("@somechannel.jpg".to_string()),
            ),
        });

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["source"]["avatar_filename"], "@somechannel.jpg");
    }

    #[test]
    fn it_should_omit_the_channel_avatar_filename_when_absent() {
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());

        let response = RecentVideoResponse::from(RecentVideo {
            video,
            source: VideoSource::Channel(
                ChannelHandle::new("@somechannel").unwrap(),
                PlaylistPath::new("creators/somechannel").unwrap(),
                None,
            ),
        });

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["source"]["avatar_filename"], serde_json::Value::Null);
    }

    #[test]
    fn it_should_have_no_avatar_filename_for_a_playlist_source() {
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());

        let response = RecentVideoResponse::from(RecentVideo {
            video,
            source: VideoSource::Playlist(
                PlaylistId::new("PL1").unwrap(),
                PlaylistPath::new("music").unwrap(),
            ),
        });

        assert_eq!(response.source.avatar_filename, None);
    }

    #[test]
    fn it_should_include_recent_video_thumbnail_filename_when_present() {
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp())
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "My Video.mp4",
                Some("My Video.jpg".to_string()),
                None,
                fixed_timestamp(),
            );

        let response = RecentVideoResponse::from(RecentVideo {
            video,
            source: VideoSource::Playlist(
                PlaylistId::new("PL1").unwrap(),
                PlaylistPath::new("music").unwrap(),
            ),
        });

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["thumbnail_filename"], "My Video.jpg");
    }

    #[test]
    fn it_should_omit_recent_video_thumbnail_filename_when_absent() {
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());

        let response = RecentVideoResponse::from(RecentVideo {
            video,
            source: VideoSource::Playlist(
                PlaylistId::new("PL1").unwrap(),
                PlaylistPath::new("music").unwrap(),
            ),
        });

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["thumbnail_filename"], serde_json::Value::Null);
    }

    #[test]
    fn it_should_include_recent_video_duration_when_present() {
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp())
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "My Video.mp4",
                None,
                Some(223),
                fixed_timestamp(),
            );

        let response = RecentVideoResponse::from(RecentVideo {
            video,
            source: VideoSource::Playlist(
                PlaylistId::new("PL1").unwrap(),
                PlaylistPath::new("music").unwrap(),
            ),
        });

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["duration_seconds"], 223);
    }

    #[test]
    fn it_should_omit_recent_video_duration_when_absent() {
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());

        let response = RecentVideoResponse::from(RecentVideo {
            video,
            source: VideoSource::Playlist(
                PlaylistId::new("PL1").unwrap(),
                PlaylistPath::new("music").unwrap(),
            ),
        });

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["duration_seconds"], serde_json::Value::Null);
    }
}
