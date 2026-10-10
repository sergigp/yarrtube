use crate::domain::video::Video;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::sync::Arc;

/// What relocating one video did, or would do on a dry run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Relocation {
    /// The video's media moved from one recorded path to another.
    Moved {
        from: String,
        to: String,
    },
    /// The video already sits in its season folder.
    AlreadyInPlace,
    Skipped(SkipReason),
}

/// Why a video could not be relocated; reconcile heals it later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// No publish time is known (no metadata), so no number can be assigned.
    NoPublishTimestamp,
    /// The recorded media file is not on disk.
    FileMissing,
}

/// Moves one downloaded video from the legacy layout into its season folder.
/// Shared by the `migrate-layout` subcommand and by reconcile's
/// missing-metadata recovery for videos the migration skipped.
pub struct VideoRelocator {
    video_repository: Arc<dyn VideoRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    clock: Arc<dyn Clock>,
}

impl VideoRelocator {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_repository,
            video_file_repository,
            video_metadata_repository,
            clock,
        }
    }
}

pub trait VideoRelocatorApi: Send + Sync {
    /// Assigns the number from `published_at` if absent, moves
    /// media/thumbnail/nfo into the season folder, rewrites the NFO, records
    /// the new filenames. On a dry run, reports what it would do and changes
    /// nothing.
    fn relocate(
        &self,
        video: &Video,
        published_at: Option<DateTime<Utc>>,
        output_dir: &Path,
        dry_run: bool,
    ) -> anyhow::Result<Relocation>;
}

impl VideoRelocatorApi for VideoRelocator {
    fn relocate(
        &self,
        _video: &Video,
        _published_at: Option<DateTime<Utc>>,
        _output_dir: &Path,
        _dry_run: bool,
    ) -> anyhow::Result<Relocation> {
        Ok(Relocation::AlreadyInPlace)
    }
}
