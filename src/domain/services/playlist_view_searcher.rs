use crate::domain::playlist::{Playlist, PlaylistId, PlaylistView};
use crate::domain::playlist_video::PlaylistVideo;
use crate::domain::video::VideoRecordId;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// Reads playlists as `PlaylistView`s.
#[derive(Clone)]
pub struct PlaylistViewSearcher {
    repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    #[allow(dead_code)]
    task_repository: Arc<dyn TaskRepository>,
    #[allow(dead_code)]
    videos_root: PathBuf,
}

impl PlaylistViewSearcher {
    pub fn new(
        repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        task_repository: Arc<dyn TaskRepository>,
        videos_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            repository,
            playlist_video_repository,
            video_repository,
            task_repository,
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
        let counts = self.unwatched_counts()?;
        Ok(playlists
            .into_iter()
            .map(|playlist| Self::playlist_view(playlist, &counts))
            .collect())
    }
}

impl PlaylistViewSearcher {
    /// Downloaded, unwatched videos per playlist. Playlists with none are absent.
    fn unwatched_counts(&self) -> anyhow::Result<HashMap<PlaylistId, usize>> {
        let playlist_videos = self.playlist_video_repository.list()?;
        let unwatched = self.unwatched_video_ids(&playlist_videos)?;
        Ok(playlist_videos
            .into_iter()
            .filter(|playlist_video| unwatched.contains(&playlist_video.video_id))
            .fold(HashMap::new(), |mut counts, playlist_video| {
                *counts.entry(playlist_video.playlist_id).or_insert(0) += 1;
                counts
            }))
    }

    fn unwatched_video_ids(
        &self,
        playlist_videos: &[PlaylistVideo],
    ) -> anyhow::Result<HashSet<VideoRecordId>> {
        let video_ids: Vec<VideoRecordId> = playlist_videos
            .iter()
            .map(|playlist_video| playlist_video.video_id.clone())
            .collect();
        Ok(self
            .video_repository
            .find_many(&video_ids)?
            .into_iter()
            .filter(|video| video.is_downloaded_and_unwatched())
            .map(|video| video.id)
            .collect())
    }

    fn playlist_view(playlist: Playlist, counts: &HashMap<PlaylistId, usize>) -> PlaylistView {
        PlaylistView {
            work_in_progress: false,
            unwatched_count: counts.get(&playlist.id).copied().unwrap_or(0),
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
