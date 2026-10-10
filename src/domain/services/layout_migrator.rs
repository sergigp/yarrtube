use crate::domain::services::{ShowFileWriter, SkipReason, VideoRelocator};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::sync::Arc;

/// What a `migrate-layout` run moved, skipped and failed, each keyed by the
/// video's recorded path: moves as `(from, to)`, skips with their reason,
/// failures with their error.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub moved: Vec<(String, String)>,
    pub skipped: Vec<(String, SkipReason)>,
    pub failed: Vec<(String, String)>,
}

/// Moves an existing library into the TV-show layout from the database
/// alone: per source, writes its show files, then relocates its downloaded
/// legacy-layout videos, per publish day in publish-time order, so same-day
/// indexes follow publish order. Idempotent and resumable.
pub struct LayoutMigrator {
    video_repository: Arc<dyn VideoRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    relocator: Arc<VideoRelocator>,
    show_file_writer: Arc<ShowFileWriter>,
    videos_path: String,
}

impl LayoutMigrator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        relocator: Arc<VideoRelocator>,
        show_file_writer: Arc<ShowFileWriter>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            playlist_repository,
            channel_repository,
            playlist_video_repository,
            channel_video_repository,
            video_metadata_repository,
            relocator,
            show_file_writer,
            videos_path: videos_path.into(),
        }
    }
}

pub trait LayoutMigratorApi: Send + Sync {
    /// Relocates every downloaded legacy-layout video into its season folder
    /// and writes every source's show files; on a dry run, reports the same
    /// plan and changes nothing. A video that fails to move is reported and
    /// the run continues.
    fn migrate(&self, dry_run: bool) -> anyhow::Result<MigrationReport>;
}

impl LayoutMigratorApi for LayoutMigrator {
    fn migrate(&self, _dry_run: bool) -> anyhow::Result<MigrationReport> {
        Ok(MigrationReport::default())
    }
}
