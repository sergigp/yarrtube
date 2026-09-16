use crate::domain::video::Video;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::shared::{Quality, VideoId};

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
}
