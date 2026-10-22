use crate::domain::event::ScheduledEvent;
use crate::domain::playlist::{Playlist, PlaylistId, PlaylistView};
use crate::domain::show::has_work_in_progress;
use crate::domain::task::ScheduledTask;
use crate::domain::video::{Video, VideoRecordId};
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::domain_events::event_repository::EventRepository;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Reads playlists as `PlaylistView`s.
#[derive(Clone)]
pub struct PlaylistViewSearcher {
    repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    task_repository: Arc<dyn TaskRepository>,
    event_repository: Arc<dyn EventRepository>,
    videos_root: PathBuf,
}

impl PlaylistViewSearcher {
    pub fn new(
        repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        task_repository: Arc<dyn TaskRepository>,
        event_repository: Arc<dyn EventRepository>,
        videos_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            repository,
            playlist_video_repository,
            video_repository,
            task_repository,
            event_repository,
            videos_root: videos_root.into(),
        }
    }
}

pub trait PlaylistViewSearcherApi: Send + Sync {
    fn search_all(&self) -> anyhow::Result<Vec<PlaylistView>>;
}

impl PlaylistViewSearcherApi for PlaylistViewSearcher {
    fn search_all(&self) -> anyhow::Result<Vec<PlaylistView>> {
        let playlists = self.repository.list()?;
        let videos = self.videos_by_playlist()?;
        let tasks = self.task_repository.list_non_completed()?;
        let events = self.event_repository.list_eligible()?;
        Ok(playlists
            .into_iter()
            .map(|playlist| {
                let playlist_videos = videos.get(&playlist.id).map(Vec::as_slice).unwrap_or(&[]);
                self.playlist_view(playlist, playlist_videos, &tasks, &events)
            })
            .collect())
    }
}

impl PlaylistViewSearcher {
    /// Every stored video, grouped by the playlist it belongs to. Playlists
    /// with none are absent.
    fn videos_by_playlist(&self) -> anyhow::Result<HashMap<PlaylistId, Vec<Video>>> {
        let playlist_videos = self.playlist_video_repository.list()?;
        let video_ids: Vec<VideoRecordId> = playlist_videos
            .iter()
            .map(|playlist_video| playlist_video.video_id.clone())
            .collect();
        let videos: HashMap<VideoRecordId, Video> = self
            .video_repository
            .find_many(&video_ids)?
            .into_iter()
            .map(|video| (video.id.clone(), video))
            .collect();
        Ok(playlist_videos
            .into_iter()
            .filter_map(|playlist_video| {
                let video = videos.get(&playlist_video.video_id)?.clone();
                Some((playlist_video.playlist_id, video))
            })
            .fold(HashMap::new(), |mut grouped, (playlist_id, video)| {
                grouped.entry(playlist_id).or_default().push(video);
                grouped
            }))
    }

    fn playlist_view(
        &self,
        playlist: Playlist,
        videos: &[Video],
        tasks: &[ScheduledTask],
        events: &[ScheduledEvent],
    ) -> PlaylistView {
        let show_dir = self.videos_root.join(playlist.path.as_str());
        PlaylistView {
            unwatched_count: videos
                .iter()
                .filter(|video| video.is_downloaded_and_unwatched())
                .count(),
            work_in_progress: has_work_in_progress(videos, tasks, events, &show_dir),
            id: playlist.id,
            name: playlist.name,
            path: playlist.path,
            quality: playlist.quality,
            kind: playlist.kind,
            exclude_from_home: playlist.exclude_from_home,
            hide_unwatched_count: playlist.hide_unwatched_count,
            reconcile_interval: playlist.reconcile_interval,
            created_at: playlist.created_at,
        }
    }
}
