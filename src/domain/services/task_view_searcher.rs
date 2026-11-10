use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::task::{ScheduledTask, Task, TaskView};
use crate::domain::video::VideoId;
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
}

pub trait TaskViewSearcherApi: Send + Sync {
    fn search_all_pending(&self) -> anyhow::Result<Vec<TaskView>>;
}

impl TaskViewSearcherApi for TaskViewSearcher {
    fn search_all_pending(&self) -> anyhow::Result<Vec<TaskView>> {
        self.task_repository
            .list_non_completed()?
            .into_iter()
            .map(|task| self.to_view(task))
            .collect()
    }
}

impl TaskViewSearcher {
    fn to_view(&self, task: ScheduledTask) -> anyhow::Result<TaskView> {
        let payload = self.resolve_payload(&task.task)?;
        Ok(TaskView {
            id: task.id,
            task_type: task.task.task_type().to_string(),
            status: task.status,
            retries: task.retries,
            run_at: task.run_at,
            created_at: task.created_at,
            last_error: task.last_error,
            payload,
        })
    }

    fn resolve_payload(&self, task: &Task) -> anyhow::Result<HashMap<String, String>> {
        match task {
            Task::ReconcilePlaylist { playlist_id } => self.resolve_reconcile_playlist(playlist_id),
            Task::ReconcileChannel { channel_id } => self.resolve_reconcile_channel(channel_id),
            Task::DownloadVideo { video_id, .. }
            | Task::FetchThumbnail { video_id, .. }
            | Task::FetchSubtitles { video_id, .. }
            | Task::FetchChapters { video_id }
            | Task::SummarizeVideo { video_id }
            | Task::CategorizeVideo { video_id }
            | Task::UpgradeQuality { video_id, .. }
            | Task::SyncVideo { video_id } => self.resolve_video(VideoId::new(video_id.clone())?),
            Task::DeleteVideoFile { filename, .. } => Ok(filename
                .iter()
                .map(|filename| ("filename".to_string(), filename.clone()))
                .collect()),
            Task::DeletePlaylistFiles { path, .. } | Task::DeleteChannelFiles { path, .. } => {
                Ok(HashMap::from([("path".to_string(), path.clone())]))
            }
            Task::UpdateYtdlp | Task::SyncPlexLabels | Task::Unreadable { .. } => {
                Ok(HashMap::new())
            }
        }
    }

    fn resolve_reconcile_playlist(
        &self,
        playlist_id: &str,
    ) -> anyhow::Result<HashMap<String, String>> {
        let mut view = HashMap::new();
        let playlist_id = PlaylistId::new(playlist_id.to_string())?;
        view.insert("playlist_id".to_string(), playlist_id.as_str().to_string());
        if let Some(playlist) = self.playlist_repository.find(&playlist_id)? {
            view.insert(
                "playlist_name".to_string(),
                playlist.name.as_str().to_string(),
            );
        }
        Ok(view)
    }

    fn resolve_reconcile_channel(
        &self,
        channel_id: &str,
    ) -> anyhow::Result<HashMap<String, String>> {
        let mut view = HashMap::new();
        let channel_id = ChannelHandle::new(channel_id.to_string())?;
        view.insert("channel_id".to_string(), channel_id.as_str().to_string());
        if let Some(channel) = self.channel_repository.find(&channel_id)? {
            view.insert("channel_name".to_string(), channel.name.clone());
        }
        Ok(view)
    }

    /// The video's title, plus the name of the playlist or channel it
    /// belongs to.
    fn resolve_video(&self, video_id: VideoId) -> anyhow::Result<HashMap<String, String>> {
        let mut view = HashMap::new();

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
}
