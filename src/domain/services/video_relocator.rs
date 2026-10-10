use crate::domain::video::video_filename::episode_relative_path;
use crate::domain::video::{EpisodeNumber, Video, nfo_path_for, top_level_entry};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::MOVIE_NFO_FILENAME;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::sync::Arc;

/// What relocating one video did, or would do on a dry run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Relocation {
    /// The video's media moved from one recorded path to another, under
    /// its episode number.
    Moved {
        from: String,
        to: String,
        number: EpisodeNumber,
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
    /// the new filenames.
    fn relocate(
        &self,
        video: &Video,
        published_at: Option<DateTime<Utc>>,
        output_dir: &Path,
    ) -> anyhow::Result<Relocation>;

    /// Dry run: what `relocate` would do, numbered from the video's next
    /// episode number and past the numbers already `planned` this run;
    /// changes nothing.
    fn plan(
        &self,
        video: &Video,
        published_at: Option<DateTime<Utc>>,
        output_dir: &Path,
        planned: &[EpisodeNumber],
    ) -> anyhow::Result<Relocation>;
}

impl VideoRelocatorApi for VideoRelocator {
    fn relocate(
        &self,
        video: &Video,
        published_at: Option<DateTime<Utc>>,
        output_dir: &Path,
    ) -> anyhow::Result<Relocation> {
        let (from, published_at) = match self.placeable(video, published_at, output_dir) {
            Ok(placeable) => placeable,
            Err(outcome) => return Ok(outcome),
        };
        let number = self
            .video_repository
            .assign_episode_number(&video.id, published_at)?;
        let files = RelocatedFiles::new(video, &from, &number);
        self.move_files(video, &files, output_dir)?;
        self.rewrite_nfo(video, &files, &number, output_dir)?;
        self.discard_legacy_folder(&from, output_dir)?;
        self.record_relocated(video, &files, number)?;
        Ok(Relocation::Moved {
            from,
            to: files.filename,
            number,
        })
    }

    fn plan(
        &self,
        video: &Video,
        published_at: Option<DateTime<Utc>>,
        output_dir: &Path,
        planned: &[EpisodeNumber],
    ) -> anyhow::Result<Relocation> {
        let (from, published_at) = match self.placeable(video, published_at, output_dir) {
            Ok(placeable) => placeable,
            Err(outcome) => return Ok(outcome),
        };
        let next = self
            .video_repository
            .next_episode_number(&video.id, published_at)?;
        let number = past_planned(next, planned)?;
        let to = RelocatedFiles::new(video, &from, &number).filename;
        Ok(Relocation::Moved { from, to, number })
    }
}

impl VideoRelocator {
    /// The legacy video's recorded file and publish time when it can be
    /// relocated, else the outcome to report instead.
    fn placeable(
        &self,
        video: &Video,
        published_at: Option<DateTime<Utc>>,
        output_dir: &Path,
    ) -> Result<(String, DateTime<Utc>), Relocation> {
        let Some(from) = video
            .filename
            .clone()
            .filter(|_| video.is_in_legacy_layout())
        else {
            return Err(Relocation::AlreadyInPlace);
        };
        let Some(published_at) = published_at else {
            return Err(Relocation::Skipped(SkipReason::NoPublishTimestamp));
        };
        if !self.video_file_repository.file_exists(output_dir, &from) {
            return Err(Relocation::Skipped(SkipReason::FileMissing));
        }
        Ok((from, published_at))
    }

    fn move_files(
        &self,
        video: &Video,
        files: &RelocatedFiles,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let from = video.filename.iter().chain(&video.thumbnail_filename);
        let to = std::iter::once(&files.filename).chain(&files.thumbnail_filename);
        from.zip(to).try_for_each(|(from, to)| {
            self.video_file_repository
                .move_file(&output_dir.join(from), &output_dir.join(to))
        })
    }

    /// Records the video's metadata again as an episode NFO next to its
    /// relocated media, referencing the relocated thumbnail. A video with
    /// no recorded metadata yet has no NFO to rewrite.
    fn rewrite_nfo(
        &self,
        video: &Video,
        files: &RelocatedFiles,
        number: &EpisodeNumber,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let Some(metadata) = self.video_metadata_repository.find(&video.id)? else {
            return Ok(());
        };
        let thumb = files
            .thumbnail_filename
            .as_deref()
            .and_then(|f| Path::new(f).file_name())
            .map(|f| f.to_string_lossy().into_owned());
        self.video_metadata_repository.save(
            &video.id,
            &metadata.with_thumb(thumb),
            number,
            &output_dir.join(nfo_path_for(&files.filename)),
        )
    }

    /// Removes the legacy per-video folder's `movie.nfo`, then the folder
    /// itself once empty; a flat legacy file has no folder.
    fn discard_legacy_folder(&self, from: &str, output_dir: &Path) -> anyhow::Result<()> {
        let folder = top_level_entry(from);
        if folder == from {
            return Ok(());
        }
        let folder = output_dir.join(folder);
        self.video_metadata_repository
            .remove_nfo(&folder.join(MOVIE_NFO_FILENAME))?;
        self.video_file_repository.remove_if_empty(&folder)
    }

    fn record_relocated(
        &self,
        video: &Video,
        files: &RelocatedFiles,
        number: EpisodeNumber,
    ) -> anyhow::Result<()> {
        let relocated = video.clone().relocate_to_episode(
            number,
            files.filename.clone(),
            files.thumbnail_filename.clone(),
            self.clock.now(),
        );
        self.video_repository.update(&relocated)
    }
}

/// `next`, or the index after the last one `planned` on the same day when
/// that is later: what assigning would give once those were recorded.
fn past_planned(next: EpisodeNumber, planned: &[EpisodeNumber]) -> anyhow::Result<EpisodeNumber> {
    let last_planned = planned
        .iter()
        .filter(|number| {
            number.season() == next.season() && number.same_day_prefix() == next.same_day_prefix()
        })
        .map(EpisodeNumber::episode)
        .max();
    match last_planned {
        Some(last) if last >= next.episode() => Ok(EpisodeNumber::new(next.season(), last + 1)?),
        _ => Ok(next),
    }
}

/// A legacy video's media and thumbnail paths in its season folder,
/// relative to the show directory.
struct RelocatedFiles {
    filename: String,
    thumbnail_filename: Option<String>,
}

impl RelocatedFiles {
    fn new(video: &Video, from: &str, number: &EpisodeNumber) -> Self {
        let extension = Path::new(from)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("mp4");
        Self {
            filename: episode_relative_path(number, &video.title, extension),
            thumbnail_filename: video
                .thumbnail_filename
                .as_ref()
                .map(|_| episode_relative_path(number, &video.title, "jpg")),
        }
    }
}
