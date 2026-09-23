pub mod dto;

use super::blocking::run_blocking;
use super::error::ApiError;
use crate::domain::services::TaskViewSearcher;
use axum::Json;
use axum::extract::State;
use dto::TaskResponse;

pub async fn list_tasks(
    State(task_view_searcher): State<TaskViewSearcher>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    let views = run_blocking(move || task_view_searcher.search_all_pending())
        .await?
        .map_err(ApiError::internal)?;

    Ok(Json(views.into_iter().map(TaskResponse::from).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::shared::{PlaylistId, Quality, VideoId, VideoRecordId};
    use crate::domain::task::Task;
    use crate::domain::video::{Video, VideoStatus};
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
    use crate::infrastructure::repositories::sqlite_task_repository::{
        FakeTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;
    use std::sync::Arc;

    #[tokio::test]
    async fn it_should_return_an_empty_list_when_no_tasks_exist() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[]),
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(response, Ok(vec![]));
    }

    #[tokio::test]
    async fn it_should_include_a_pending_task_whose_run_at_is_in_the_future() {
        let future = fixed_timestamp() + chrono::Duration::seconds(60);
        let task_repository = task_repository_with(&[]);
        task_repository
            .schedule(&Task::UpdateYtdlp, future)
            .unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository,
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![TaskResponse {
                run_at: future,
                ..pending_task("update_ytdlp", &[])
            }])
        );
    }

    #[tokio::test]
    async fn it_should_include_a_running_task() {
        let task_repository = task_repository_with(&[Task::UpdateYtdlp]);
        let running = task_repository.list_non_completed().unwrap()[0]
            .clone()
            .start(fixed_timestamp());
        task_repository.update(&running).unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository,
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![TaskResponse {
                status: "running".to_string(),
                ..pending_task("update_ytdlp", &[])
            }])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_playlist_name_for_a_pending_reconcile_playlist_task() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[Task::ReconcilePlaylist {
                playlist_id: "PL1".to_string(),
            }]),
            playlist_repository_with(&[playlist("PL1", "My Playlist")]),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "reconcile_playlist",
                &[("playlist_name", "My Playlist")]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_payload_when_the_referenced_playlist_no_longer_exists() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[Task::ReconcilePlaylist {
                playlist_id: "PL1".to_string(),
            }]),
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(response, Ok(vec![pending_task("reconcile_playlist", &[])]));
    }

    #[tokio::test]
    async fn it_should_include_the_channel_name_for_a_pending_reconcile_channel_task() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[Task::ReconcileChannel {
                channel_id: "@somechannel".to_string(),
            }]),
            Arc::new(FakePlaylistRepository::default()),
            channel_repository_with(&[channel("@somechannel", "Some Channel")]),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "reconcile_channel",
                &[("channel_name", "Some Channel")]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_payload_when_the_referenced_channel_no_longer_exists() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[Task::ReconcileChannel {
                channel_id: "@somechannel".to_string(),
            }]),
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(response, Ok(vec![pending_task("reconcile_channel", &[])]));
    }

    #[tokio::test]
    async fn it_should_include_the_video_title_and_playlist_name_for_a_pending_download_video_task()
    {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository =
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone()));
        video_repository.save(&video("rec1", "My Video")).unwrap();
        playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                VideoRecordId::new("rec1").unwrap(),
                fixed_timestamp(),
            ))
            .unwrap();
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[download_video_task("rec1")]),
            playlist_repository_with(&[playlist("PL1", "My Playlist")]),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            playlist_video_repository,
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "download_video",
                &[
                    ("video_title", "My Video"),
                    ("playlist_name", "My Playlist")
                ]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_video_title_and_channel_name_for_a_channel_owned_download_video_task()
     {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let channel_video_repository =
            Arc::new(FakeChannelVideoRepository::new(video_repository.clone()));
        video_repository.save(&video("rec1", "My Video")).unwrap();
        channel_video_repository
            .save(&ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                VideoRecordId::new("rec1").unwrap(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[download_video_task("rec1")]),
            Arc::new(FakePlaylistRepository::default()),
            channel_repository_with(&[channel("@somechannel", "Some Channel")]),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository)),
            channel_video_repository,
        );

        let response = list(task_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "download_video",
                &[
                    ("video_title", "My Video"),
                    ("channel_name", "Some Channel")
                ]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_payload_when_a_download_video_task_references_an_untracked_video()
     {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[download_video_task("rec1")]),
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(response, Ok(vec![pending_task("download_video", &[])]));
    }

    #[tokio::test]
    async fn it_should_include_the_filename_for_a_pending_delete_video_file_task() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[Task::DeleteVideoFile {
                filename: Some("My Video.mp4".to_string()),
                thumbnail_filename: Some("My Video.jpg".to_string()),
                output_dir: "/videos/music".to_string(),
            }]),
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "delete_video_file",
                &[("filename", "My Video.mp4")]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_payload_for_a_delete_video_file_task_with_no_recorded_filename()
     {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[Task::DeleteVideoFile {
                filename: None,
                thumbnail_filename: None,
                output_dir: "/videos/music".to_string(),
            }]),
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(response, Ok(vec![pending_task("delete_video_file", &[])]));
    }

    #[tokio::test]
    async fn it_should_include_the_path_for_a_pending_delete_playlist_files_task() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[Task::DeletePlaylistFiles {
                playlist_id: "PL1".to_string(),
                path: "music/chill".to_string(),
            }]),
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "delete_playlist_files",
                &[("path", "music/chill")]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_include_the_path_for_a_pending_delete_channel_files_task() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[Task::DeleteChannelFiles {
                channel_id: "@somechannel".to_string(),
                path: "creators/somechannel".to_string(),
            }]),
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(
            response,
            Ok(vec![pending_task(
                "delete_channel_files",
                &[("path", "creators/somechannel")]
            )])
        );
    }

    #[tokio::test]
    async fn it_should_return_an_empty_payload_for_an_update_ytdlp_task() {
        let video_repository = Arc::new(FakeVideoRepository::default());
        let task_view_searcher = TaskViewSearcher::new(
            task_repository_with(&[Task::UpdateYtdlp]),
            Arc::new(FakePlaylistRepository::default()),
            Arc::new(FakeChannelRepository::default()),
            video_repository.clone(),
            Arc::new(FakePlaylistVideoRepository::new(video_repository.clone())),
            Arc::new(FakeChannelVideoRepository::new(video_repository)),
        );

        let response = list(task_view_searcher).await;

        assert_eq!(response, Ok(vec![pending_task("update_ytdlp", &[])]));
    }

    fn task_repository_with(tasks: &[Task]) -> Arc<FakeTaskRepository> {
        let repository = FakeTaskRepository::new(Arc::new(FixedClock(fixed_timestamp())));
        for task in tasks {
            repository.schedule(task, fixed_timestamp()).unwrap();
        }
        Arc::new(repository)
    }

    fn playlist_repository_with(playlists: &[Playlist]) -> Arc<FakePlaylistRepository> {
        let repository = FakePlaylistRepository::default();
        for playlist in playlists {
            repository.insert(playlist).unwrap();
        }
        Arc::new(repository)
    }

    fn channel_repository_with(channels: &[Channel]) -> Arc<FakeChannelRepository> {
        let repository = FakeChannelRepository::default();
        for channel in channels {
            repository.insert(channel).unwrap();
        }
        Arc::new(repository)
    }

    fn download_video_task(video_id: &str) -> Task {
        Task::DownloadVideo {
            video_id: video_id.to_string(),
            quality: "high".to_string(),
            output_dir: "/videos/my-playlist".to_string(),
        }
    }

    fn pending_task(task_type: &str, payload: &[(&str, &str)]) -> TaskResponse {
        TaskResponse {
            id: 1,
            task_type: task_type.to_string(),
            status: "pending".to_string(),
            retries: 0,
            run_at: fixed_timestamp(),
            created_at: fixed_timestamp(),
            last_error: None,
            payload: payload
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<HashMap<_, _>>(),
        }
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn playlist(id: &str, name: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new(name).unwrap(),
            PlaylistPath::new("my-playlist").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            fixed_timestamp(),
        )
    }

    fn channel(id: &str, name: &str) -> Channel {
        Channel::create(
            ChannelHandle::new(id).unwrap(),
            name,
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("creators/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    fn video(id: &str, title: &str) -> Video {
        Video {
            id: VideoRecordId::new(id).unwrap(),
            youtube_id: VideoId::new("yt1").unwrap(),
            title: title.to_string(),
            status: VideoStatus::Pending,
            quality: None,
            filename: None,
            thumbnail_filename: None,
            duration_seconds: None,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
        }
    }

    async fn list(task_view_searcher: TaskViewSearcher) -> Result<Vec<TaskResponse>, ApiError> {
        list_tasks(State(task_view_searcher))
            .await
            .map(|Json(tasks)| tasks)
    }
}
