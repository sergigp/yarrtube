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
