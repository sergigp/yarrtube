pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::channel::ChannelHandle;
use crate::domain::services::VideoSearcher;
use crate::domain::shared::PlaylistId;
use crate::domain::video::ListVideosError;
use axum::Json;
use axum::extract::{Path, Query, State};
use dto::{RecentVideoResponse, VideoResponse};
use serde::Deserialize;

const DEFAULT_RECENT_VIDEOS_LIMIT: usize = 20;
const MAX_RECENT_VIDEOS_LIMIT: usize = 100;

#[derive(Debug, Deserialize)]
pub struct ListRecentVideosQuery {
    #[serde(default)]
    pub limit: Option<usize>,
}

pub async fn list_videos_for_playlist(
    State(video_searcher): State<VideoSearcher>,
    Path(playlist_id): Path<String>,
) -> Result<Json<Vec<VideoResponse>>, ApiError> {
    let playlist_id = PlaylistId::new(playlist_id).map_err(ApiError::bad_request)?;

    let videos = run_blocking(move || video_searcher.list(&playlist_id))
        .await?
        .map_err(list_videos_error)?;
    Ok(Json(videos.into_iter().map(VideoResponse::from).collect()))
}

pub async fn list_videos_for_channel(
    State(video_searcher): State<VideoSearcher>,
    Path(handle): Path<String>,
) -> Result<Json<Vec<VideoResponse>>, ApiError> {
    let channel_id = ChannelHandle::new(handle).map_err(ApiError::bad_request)?;

    let videos = run_blocking(move || video_searcher.list_for_channel(&channel_id))
        .await?
        .map_err(list_videos_error)?;
    Ok(Json(videos.into_iter().map(VideoResponse::from).collect()))
}

pub async fn list_recent_videos(
    State(video_searcher): State<VideoSearcher>,
    Query(query): Query<ListRecentVideosQuery>,
) -> Result<Json<Vec<RecentVideoResponse>>, ApiError> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_RECENT_VIDEOS_LIMIT)
        .min(MAX_RECENT_VIDEOS_LIMIT);

    let videos = run_blocking(move || video_searcher.list_recent(limit))
        .await?
        .map_err(list_videos_error)?;
    Ok(Json(
        videos.into_iter().map(RecentVideoResponse::from).collect(),
    ))
}

fn list_videos_error(error: ListVideosError) -> ApiError {
    match error {
        e @ ListVideosError::PlaylistNotFound(_) => ApiError::bad_request(e),
        e @ ListVideosError::ChannelNotFound(_) => ApiError::bad_request(e),
        e @ ListVideosError::Repository(_) => ApiError::internal(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::shared::{Quality, VideoId};
    use crate::domain::video::Video;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, FakeChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, FakeChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
        FakePlaylistVideoRepository, PlaylistVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
    use chrono::{DateTime, Utc};
    use dto::RecentVideoSourceResponse;
    use std::sync::Arc;

    #[tokio::test]
    async fn it_should_return_the_playlists_videos() {
        let repositories = Repositories::with_playlist("PL1");
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp());
        repositories.add_to_playlist("PL1", &video);

        let response = list_for_playlist(repositories, "PL1").await;

        assert_eq!(
            response,
            Ok(vec![pending_video_response("vid1", "My Video")])
        );
    }

    #[tokio::test]
    async fn it_should_return_the_download_details_of_a_downloaded_video() {
        let repositories = Repositories::with_playlist("PL1");
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp())
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "My Video.mp4",
                Some("My Video.jpg".to_string()),
                Some(223),
                fixed_timestamp(),
            );
        repositories.add_to_playlist("PL1", &video);

        let response = list_for_playlist(repositories, "PL1").await;

        assert_eq!(
            response,
            Ok(vec![VideoResponse {
                status: "DOWNLOADED".to_string(),
                quality: Some("high".to_string()),
                filename: Some("My Video.mp4".to_string()),
                thumbnail_filename: Some("My Video.jpg".to_string()),
                duration_seconds: Some(223),
                ..pending_video_response("vid1", "My Video")
            }])
        );
    }

    #[tokio::test]
    async fn it_should_return_videos_ordered_by_playlist_position() {
        let repositories = Repositories::with_playlist("PL1");
        for (youtube_id, title, position) in [
            ("vid_third", "Third", 2),
            ("vid_first", "First", 0),
            ("vid_second", "Second", 1),
        ] {
            let video = Video::create(VideoId::new(youtube_id).unwrap(), title, fixed_timestamp());
            repositories.videos.save(&video).unwrap();
            repositories
                .playlist_videos
                .save(&PlaylistVideo::create_with_position(
                    PlaylistId::new("PL1").unwrap(),
                    video.id.clone(),
                    position,
                    fixed_timestamp(),
                ))
                .unwrap();
        }

        let response = list_for_playlist(repositories, "PL1").await;

        assert_eq!(
            response,
            Ok(vec![
                pending_video_response("vid_first", "First"),
                pending_video_response("vid_second", "Second"),
                pending_video_response("vid_third", "Third"),
            ])
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_list_when_the_playlist_has_no_videos() {
        let response = list_for_playlist(Repositories::with_playlist("PL1"), "PL1").await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_playlist_does_not_exist() {
        let response = list_for_playlist(Repositories::default(), "PL404").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("playlist PL404 not found"))
        );
    }

    #[tokio::test]
    async fn it_should_return_the_channels_videos_ordered_by_recency() {
        let repositories = Repositories::with_channel(channel("@somechannel", None));
        for (youtube_id, title, position) in
            [("vid_newest", "Newest", 0), ("vid_oldest", "Oldest", 1)]
        {
            let video = Video::create(VideoId::new(youtube_id).unwrap(), title, fixed_timestamp());
            repositories.add_to_channel("@somechannel", &video, position);
        }

        let response = list_for_channel(repositories, "@somechannel").await;

        assert_eq!(
            response,
            Ok(vec![
                pending_video_response("vid_newest", "Newest"),
                pending_video_response("vid_oldest", "Oldest"),
            ])
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_list_when_the_channel_has_no_videos() {
        let response = list_for_channel(
            Repositories::with_channel(channel("@somechannel", None)),
            "@somechannel",
        )
        .await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_return_400_when_the_channel_does_not_exist() {
        let response = list_for_channel(Repositories::default(), "@missing").await;

        assert_eq!(
            response,
            Err(ApiError::bad_request("channel @missing not found"))
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_list_when_no_downloaded_videos_exist() {
        let response = list_recent(Repositories::default(), None).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_combine_and_sort_recent_videos_from_playlists_and_channels() {
        let repositories = Repositories::with_playlist("PL1");
        repositories
            .channels
            .insert(&channel("@somechannel", None))
            .unwrap();
        repositories.add_to_playlist(
            "PL1",
            &downloaded_video("vid_from_playlist", "From Playlist", None, 100),
        );
        repositories.add_to_channel(
            "@somechannel",
            &downloaded_video(
                "vid_from_channel",
                "From Channel",
                Some("From Channel.jpg"),
                200,
            ),
            0,
        );

        let response = list_recent(repositories, None).await;

        assert_eq!(
            response,
            Ok(vec![
                RecentVideoResponse {
                    thumbnail_filename: Some("From Channel.jpg".to_string()),
                    ..recent_video_response(
                        "vid_from_channel",
                        "From Channel",
                        channel_source(None)
                    )
                },
                recent_video_response("vid_from_playlist", "From Playlist", playlist_source()),
            ])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_duration_of_a_recent_video() {
        let repositories = Repositories::with_playlist("PL1");
        let video = Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "My Video.mp4",
                None,
                Some(223),
                fixed_timestamp(),
            );
        repositories.add_to_playlist("PL1", &video);

        let response = list_recent(repositories, None).await;

        assert_eq!(
            response,
            Ok(vec![RecentVideoResponse {
                duration_seconds: Some(223),
                ..recent_video_response("vid1", "My Video", playlist_source())
            }])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_channel_avatar_filename_in_a_recent_videos_source() {
        let repositories =
            Repositories::with_channel(channel("@somechannel", Some("@somechannel.jpg")));
        repositories.add_to_channel(
            "@somechannel",
            &downloaded_video("vid1", "My Video", None, 100),
            0,
        );

        let response = list_recent(repositories, None).await;

        assert_eq!(
            response,
            Ok(vec![recent_video_response(
                "vid1",
                "My Video",
                channel_source(Some("@somechannel.jpg"))
            )])
        );
    }

    #[tokio::test]
    async fn it_should_exclude_non_downloaded_videos_from_recent() {
        let repositories = Repositories::with_playlist("PL1");
        repositories.add_to_playlist(
            "PL1",
            &Video::create(
                VideoId::new("vid_pending").unwrap(),
                "Pending",
                fixed_timestamp(),
            ),
        );

        let response = list_recent(repositories, None).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_list_a_video_tracked_by_two_sources_once_per_source() {
        let repositories = Repositories::with_playlist("PL1");
        repositories
            .channels
            .insert(&channel("@somechannel", None))
            .unwrap();
        let shared = downloaded_video("vid_shared", "Shared", None, 100);
        repositories.add_to_playlist("PL1", &shared);
        repositories
            .channel_videos
            .save(&ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                shared.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();

        let response = list_recent(repositories, None).await;

        assert_eq!(
            response,
            Ok(vec![
                recent_video_response("vid_shared", "Shared", playlist_source()),
                recent_video_response("vid_shared", "Shared", channel_source(None)),
            ])
        );
    }

    #[tokio::test]
    async fn it_should_apply_the_default_limit_of_20_when_omitted() {
        let repositories = repositories_with_numbered_videos(25);

        let response = list_recent(repositories, None).await;

        assert_eq!(response, Ok(numbered_recent_videos((5..25).rev())));
    }

    #[tokio::test]
    async fn it_should_narrow_the_result_with_an_explicit_limit() {
        let repositories = repositories_with_numbered_videos(3);

        let response = list_recent(repositories, Some(2)).await;

        assert_eq!(response, Ok(numbered_recent_videos((1..3).rev())));
    }

    #[tokio::test]
    async fn it_should_cap_the_limit_at_100() {
        let repositories = repositories_with_numbered_videos(105);

        let response = list_recent(repositories, Some(1000)).await;

        assert_eq!(response, Ok(numbered_recent_videos((5..105).rev())));
    }

    #[derive(Default)]
    struct Repositories {
        playlists: FakePlaylistRepository,
        playlist_videos: FakePlaylistVideoRepository,
        channels: FakeChannelRepository,
        channel_videos: FakeChannelVideoRepository,
        videos: FakeVideoRepository,
    }

    impl Repositories {
        fn with_playlist(id: &str) -> Self {
            let repositories = Self::default();
            repositories.playlists.insert(&playlist(id)).unwrap();
            repositories
        }

        fn with_channel(channel: Channel) -> Self {
            let repositories = Self::default();
            repositories.channels.insert(&channel).unwrap();
            repositories
        }

        fn add_to_playlist(&self, playlist_id: &str, video: &Video) {
            self.videos.save(video).unwrap();
            self.playlist_videos
                .save(&PlaylistVideo::create(
                    PlaylistId::new(playlist_id).unwrap(),
                    video.id.clone(),
                    fixed_timestamp(),
                ))
                .unwrap();
        }

        fn add_to_channel(&self, handle: &str, video: &Video, position: i64) {
            self.videos.save(video).unwrap();
            self.channel_videos
                .save(&ChannelVideo::create(
                    ChannelHandle::new(handle).unwrap(),
                    video.id.clone(),
                    position,
                    fixed_timestamp(),
                ))
                .unwrap();
        }

        fn into_video_searcher(self) -> VideoSearcher {
            VideoSearcher::new(
                Arc::new(self.playlists),
                Arc::new(self.playlist_videos),
                Arc::new(self.channels),
                Arc::new(self.channel_videos),
                Arc::new(self.videos),
            )
        }
    }

    async fn list_for_playlist(
        repositories: Repositories,
        playlist_id: &str,
    ) -> Result<Vec<VideoResponse>, ApiError> {
        list_videos_for_playlist(
            State(repositories.into_video_searcher()),
            Path(playlist_id.to_string()),
        )
        .await
        .map(|Json(videos)| videos)
    }

    async fn list_for_channel(
        repositories: Repositories,
        handle: &str,
    ) -> Result<Vec<VideoResponse>, ApiError> {
        list_videos_for_channel(
            State(repositories.into_video_searcher()),
            Path(handle.to_string()),
        )
        .await
        .map(|Json(videos)| videos)
    }

    async fn list_recent(
        repositories: Repositories,
        limit: Option<usize>,
    ) -> Result<Vec<RecentVideoResponse>, ApiError> {
        list_recent_videos(
            State(repositories.into_video_searcher()),
            Query(ListRecentVideosQuery { limit }),
        )
        .await
        .map(|Json(videos)| videos)
    }

    fn repositories_with_numbered_videos(count: i64) -> Repositories {
        let repositories = Repositories::with_playlist("PL1");
        for i in 0..count {
            repositories.add_to_playlist(
                "PL1",
                &downloaded_video(&format!("vid{i}"), &format!("Video {i}"), None, i),
            );
        }
        repositories
    }

    fn numbered_recent_videos(numbers: impl Iterator<Item = i64>) -> Vec<RecentVideoResponse> {
        numbers
            .map(|i| {
                recent_video_response(&format!("vid{i}"), &format!("Video {i}"), playlist_source())
            })
            .collect()
    }

    fn pending_video_response(youtube_id: &str, title: &str) -> VideoResponse {
        VideoResponse {
            id: youtube_id.to_string(),
            title: title.to_string(),
            status: "PENDING".to_string(),
            quality: None,
            filename: None,
            thumbnail_filename: None,
            duration_seconds: None,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
        }
    }

    fn recent_video_response(
        youtube_id: &str,
        title: &str,
        source: RecentVideoSourceResponse,
    ) -> RecentVideoResponse {
        RecentVideoResponse {
            id: youtube_id.to_string(),
            title: title.to_string(),
            thumbnail_filename: None,
            duration_seconds: None,
            source,
        }
    }

    fn playlist_source() -> RecentVideoSourceResponse {
        RecentVideoSourceResponse {
            kind: "playlist".to_string(),
            id: "PL1".to_string(),
            path: "music".to_string(),
            avatar_filename: None,
        }
    }

    fn channel_source(avatar_filename: Option<&str>) -> RecentVideoSourceResponse {
        RecentVideoSourceResponse {
            kind: "channel".to_string(),
            id: "@somechannel".to_string(),
            path: "creators/somechannel".to_string(),
            avatar_filename: avatar_filename.map(str::to_string),
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn playlist(id: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new("My Playlist").unwrap(),
            PlaylistPath::new("music").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            fixed_timestamp(),
        )
    }

    fn channel(id: &str, avatar_filename: Option<&str>) -> Channel {
        Channel::create(
            ChannelHandle::new(id).unwrap(),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            avatar_filename.map(str::to_string),
            fixed_timestamp(),
        )
    }

    fn downloaded_video(
        youtube_id: &str,
        title: &str,
        thumbnail_filename: Option<&str>,
        created_at_seconds: i64,
    ) -> Video {
        let created_at = DateTime::<Utc>::from_timestamp(created_at_seconds, 0).unwrap();
        Video::create(VideoId::new(youtube_id).unwrap(), title, created_at).mark_downloaded(
            Quality::High,
            format!("{title}.mp4"),
            thumbnail_filename.map(str::to_string),
            None,
            created_at,
        )
    }
}
