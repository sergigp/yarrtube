use crate::domain::channel::ChannelHandle;
use crate::domain::shared::{PlaylistId, VideoRecordId};
use crate::domain::task::{ScheduledTask, Task, TaskView};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::collections::HashMap;
use std::sync::Arc;

/// Reads pending/running tasks, enriched with whatever context is
/// resolvable from each task's stored payload. See design.md's per-`task_type`
/// key table.
#[derive(Clone)]
pub struct TaskViewSearcher {
    task_repository: Arc<dyn TaskRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    video_repository: Arc<dyn VideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
}

impl TaskViewSearcher {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        task_repository: Arc<dyn TaskRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        video_repository: Arc<dyn VideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
    ) -> Self {
        Self {
            task_repository,
            playlist_repository,
            channel_repository,
            video_repository,
            playlist_video_repository,
            channel_video_repository,
        }
    }

    pub fn search_all_pending(&self) -> anyhow::Result<Vec<TaskView>> {
        self.task_repository
            .list_non_completed()?
            .into_iter()
            .map(|task| self.to_view(task))
            .collect()
    }

    fn to_view(&self, task: ScheduledTask) -> anyhow::Result<TaskView> {
        let payload = self.resolve_payload(&task)?;
        Ok(TaskView {
            id: task.id,
            task_type: task.task_type,
            status: task.status,
            retries: task.retries,
            run_at: task.run_at,
            created_at: task.created_at,
            last_error: task.last_error,
            payload,
        })
    }

    fn resolve_payload(&self, task: &ScheduledTask) -> anyhow::Result<HashMap<String, String>> {
        match task.task_type.as_str() {
            "reconcile_playlist" => self.resolve_reconcile_playlist(&task.payload),
            "reconcile_channel" => self.resolve_reconcile_channel(&task.payload),
            "download_video" => self.resolve_download_video(&task.payload),
            "delete_video_file" => self.resolve_delete_video_file(&task.payload),
            "delete_playlist_files" => self.resolve_delete_playlist_files(&task.payload),
            "delete_channel_files" => self.resolve_delete_channel_files(&task.payload),
            _ => Ok(HashMap::new()),
        }
    }

    fn resolve_reconcile_playlist(&self, payload: &str) -> anyhow::Result<HashMap<String, String>> {
        let mut view = HashMap::new();
        let playlist_id = PlaylistId::new(Task::decode_reconcile_playlist_payload(payload)?)?;
        if let Some(playlist) = self.playlist_repository.find(&playlist_id)? {
            view.insert(
                "playlist_name".to_string(),
                playlist.name.as_str().to_string(),
            );
        }
        Ok(view)
    }

    fn resolve_reconcile_channel(&self, payload: &str) -> anyhow::Result<HashMap<String, String>> {
        let mut view = HashMap::new();
        let channel_id = ChannelHandle::new(Task::decode_reconcile_channel_payload(payload)?)?;
        if let Some(channel) = self.channel_repository.find(&channel_id)? {
            view.insert("channel_name".to_string(), channel.name.clone());
        }
        Ok(view)
    }

    fn resolve_download_video(&self, payload: &str) -> anyhow::Result<HashMap<String, String>> {
        let mut view = HashMap::new();
        let (video_id, _quality, _output_dir) = Task::decode_download_video_payload(payload)?;
        let video_id = VideoRecordId::new(video_id)?;

        if let Some(video) = self.video_repository.find(&video_id)? {
            view.insert("video_title".to_string(), video.title.clone());
        }

        if let Some(playlist_video) = self.playlist_video_repository.find_by_video(&video_id)? {
            if let Some(playlist) = self.playlist_repository.find(&playlist_video.playlist_id)? {
                view.insert(
                    "playlist_name".to_string(),
                    playlist.name.as_str().to_string(),
                );
            }
        } else if let Some(channel_video) =
            self.channel_video_repository.find_by_video(&video_id)?
            && let Some(channel) = self.channel_repository.find(&channel_video.channel_id)?
        {
            view.insert("channel_name".to_string(), channel.name.clone());
        }

        Ok(view)
    }

    fn resolve_delete_video_file(&self, payload: &str) -> anyhow::Result<HashMap<String, String>> {
        let mut view = HashMap::new();
        let (filename, _thumbnail_filename, _output_dir) =
            Task::decode_delete_video_file_payload(payload)?;
        if let Some(filename) = filename {
            view.insert("filename".to_string(), filename);
        }
        Ok(view)
    }

    fn resolve_delete_playlist_files(
        &self,
        payload: &str,
    ) -> anyhow::Result<HashMap<String, String>> {
        let mut view = HashMap::new();
        let (_playlist_id, path) = Task::decode_delete_playlist_files_payload(payload)?;
        view.insert("path".to_string(), path);
        Ok(view)
    }

    fn resolve_delete_channel_files(
        &self,
        payload: &str,
    ) -> anyhow::Result<HashMap<String, String>> {
        let mut view = HashMap::new();
        let (_channel_id, path) = Task::decode_delete_channel_files_payload(payload)?;
        view.insert("path".to_string(), path);
        Ok(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::shared::Quality;
    use crate::domain::video::Video;
    use crate::infrastructure::repositories::sqlite_channel_repository::FakeChannelRepository;
    use crate::infrastructure::repositories::sqlite_channel_video_repository::FakeChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::FakePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::FakePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use rusqlite::Connection;
    use std::sync::Mutex;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    struct Fixture {
        task_repository: Arc<dyn TaskRepository>,
        playlist_repository: Arc<FakePlaylistRepository>,
        channel_repository: Arc<FakeChannelRepository>,
        video_repository: Arc<FakeVideoRepository>,
        playlist_video_repository: Arc<FakePlaylistVideoRepository>,
        channel_video_repository: Arc<FakeChannelVideoRepository>,
    }

    impl Fixture {
        fn new() -> Self {
            let mut conn = Connection::open_in_memory().unwrap();
            crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
            Self {
                task_repository: Arc::new(SqliteTaskRepository::new(
                    Arc::new(Mutex::new(conn)),
                    Arc::new(FixedClock(fixed_timestamp())),
                )),
                playlist_repository: Arc::new(FakePlaylistRepository::default()),
                channel_repository: Arc::new(FakeChannelRepository::default()),
                video_repository: Arc::new(FakeVideoRepository::default()),
                playlist_video_repository: Arc::new(FakePlaylistVideoRepository::default()),
                channel_video_repository: Arc::new(FakeChannelVideoRepository::default()),
            }
        }

        fn searcher(&self) -> TaskViewSearcher {
            TaskViewSearcher::new(
                self.task_repository.clone(),
                self.playlist_repository.clone(),
                self.channel_repository.clone(),
                self.video_repository.clone(),
                self.playlist_video_repository.clone(),
                self.channel_video_repository.clone(),
            )
        }
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
            youtube_id: crate::domain::shared::VideoId::new("yt1").unwrap(),
            title: title.to_string(),
            status: crate::domain::video::VideoStatus::Pending,
            quality: None,
            filename: None,
            thumbnail_filename: None,
            duration_seconds: None,
            created_at: fixed_timestamp(),
            updated_at: fixed_timestamp(),
        }
    }

    #[test]
    fn it_should_resolve_the_playlist_name_for_a_reconcile_playlist_task() {
        let fixture = Fixture::new();
        fixture
            .playlist_repository
            .insert(&playlist("PL1", "My Playlist"))
            .unwrap();
        fixture
            .task_repository
            .schedule(
                &Task::ReconcilePlaylist {
                    playlist_id: "PL1".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(
            views[0].payload.get("playlist_name"),
            Some(&"My Playlist".to_string())
        );
        assert_eq!(views[0].payload.len(), 1);
    }

    #[test]
    fn it_should_omit_the_playlist_name_when_the_playlist_no_longer_exists() {
        let fixture = Fixture::new();
        fixture
            .task_repository
            .schedule(
                &Task::ReconcilePlaylist {
                    playlist_id: "PL1".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert!(views[0].payload.is_empty());
    }

    #[test]
    fn it_should_resolve_the_channel_name_for_a_reconcile_channel_task() {
        let fixture = Fixture::new();
        fixture
            .channel_repository
            .insert(&channel("@somechannel", "Some Channel"))
            .unwrap();
        fixture
            .task_repository
            .schedule(
                &Task::ReconcileChannel {
                    channel_id: "@somechannel".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(
            views[0].payload.get("channel_name"),
            Some(&"Some Channel".to_string())
        );
        assert_eq!(views[0].payload.len(), 1);
    }

    #[test]
    fn it_should_omit_the_channel_name_when_the_channel_no_longer_exists() {
        let fixture = Fixture::new();
        fixture
            .task_repository
            .schedule(
                &Task::ReconcileChannel {
                    channel_id: "@somechannel".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert!(views[0].payload.is_empty());
    }

    #[test]
    fn it_should_resolve_the_video_title_and_playlist_name_for_a_playlist_owned_download_video_task()
     {
        let fixture = Fixture::new();
        fixture
            .playlist_repository
            .insert(&playlist("PL1", "My Playlist"))
            .unwrap();
        fixture
            .video_repository
            .save(&video("rec1", "My Video"))
            .unwrap();
        fixture
            .playlist_video_repository
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                VideoRecordId::new("rec1").unwrap(),
                fixed_timestamp(),
            ))
            .unwrap();
        fixture
            .task_repository
            .schedule(
                &Task::DownloadVideo {
                    video_id: "rec1".to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/my-playlist".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(
            views[0].payload.get("video_title"),
            Some(&"My Video".to_string())
        );
        assert_eq!(
            views[0].payload.get("playlist_name"),
            Some(&"My Playlist".to_string())
        );
        assert_eq!(views[0].payload.len(), 2);
    }

    #[test]
    fn it_should_resolve_the_video_title_and_channel_name_for_a_channel_owned_download_video_task()
    {
        let fixture = Fixture::new();
        fixture
            .channel_repository
            .insert(&channel("@somechannel", "Some Channel"))
            .unwrap();
        fixture
            .video_repository
            .save(&video("rec1", "My Video"))
            .unwrap();
        fixture
            .channel_video_repository
            .save(&ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                VideoRecordId::new("rec1").unwrap(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        fixture
            .task_repository
            .schedule(
                &Task::DownloadVideo {
                    video_id: "rec1".to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos/somechannel".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(
            views[0].payload.get("video_title"),
            Some(&"My Video".to_string())
        );
        assert_eq!(
            views[0].payload.get("channel_name"),
            Some(&"Some Channel".to_string())
        );
        assert_eq!(views[0].payload.len(), 2);
    }

    #[test]
    fn it_should_omit_every_key_when_a_download_video_task_references_an_untracked_video() {
        let fixture = Fixture::new();
        fixture
            .task_repository
            .schedule(
                &Task::DownloadVideo {
                    video_id: "rec1".to_string(),
                    quality: "high".to_string(),
                    output_dir: "/videos".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert!(views[0].payload.is_empty());
    }

    #[test]
    fn it_should_resolve_the_filename_for_a_delete_video_file_task() {
        let fixture = Fixture::new();
        fixture
            .task_repository
            .schedule(
                &Task::DeleteVideoFile {
                    filename: Some("My Video.mp4".to_string()),
                    thumbnail_filename: Some("My Video.jpg".to_string()),
                    output_dir: "/videos/music".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(
            views[0].payload.get("filename"),
            Some(&"My Video.mp4".to_string())
        );
        assert_eq!(views[0].payload.len(), 1);
    }

    #[test]
    fn it_should_omit_the_filename_for_a_delete_video_file_task_with_no_recorded_filename() {
        let fixture = Fixture::new();
        fixture
            .task_repository
            .schedule(
                &Task::DeleteVideoFile {
                    filename: None,
                    thumbnail_filename: None,
                    output_dir: "/videos/music".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert!(views[0].payload.is_empty());
    }

    #[test]
    fn it_should_resolve_the_path_for_a_delete_playlist_files_task() {
        let fixture = Fixture::new();
        fixture
            .task_repository
            .schedule(
                &Task::DeletePlaylistFiles {
                    playlist_id: "PL1".to_string(),
                    path: "music/chill".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(
            views[0].payload.get("path"),
            Some(&"music/chill".to_string())
        );
        assert_eq!(views[0].payload.len(), 1);
    }

    #[test]
    fn it_should_resolve_the_path_for_a_delete_channel_files_task() {
        let fixture = Fixture::new();
        fixture
            .task_repository
            .schedule(
                &Task::DeleteChannelFiles {
                    channel_id: "@somechannel".to_string(),
                    path: "creators/somechannel".to_string(),
                },
                fixed_timestamp(),
            )
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(
            views[0].payload.get("path"),
            Some(&"creators/somechannel".to_string())
        );
        assert_eq!(views[0].payload.len(), 1);
    }

    #[test]
    fn it_should_resolve_an_empty_payload_for_an_update_ytdlp_task() {
        let fixture = Fixture::new();
        fixture
            .task_repository
            .schedule(&Task::UpdateYtdlp, fixed_timestamp())
            .unwrap();

        let views = fixture.searcher().search_all_pending().unwrap();

        assert_eq!(views.len(), 1);
        assert!(views[0].payload.is_empty());
    }
}
