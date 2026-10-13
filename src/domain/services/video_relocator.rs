use crate::domain::video::{
    ChannelEpisodeNumber, EpisodeNumber, EpisodePaths, MOVIE_NFO_FILENAME, Video, top_level_entry,
};
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
    /// No number is recorded and no publish time is known (no metadata), so
    /// no number can be assigned.
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
    /// Uses the recorded number, else assigns one from `published_at`
    /// (skipping when neither exists), moves media/thumbnail/nfo into the
    /// season folder, rewrites the NFO, records the new filenames.
    fn relocate(
        &self,
        video: &Video,
        published_at: Option<DateTime<Utc>>,
        output_dir: &Path,
    ) -> anyhow::Result<Relocation>;

    /// Dry run: what `relocate` would do, under the recorded number, else
    /// the video's next episode number past the numbers already `planned`
    /// this run; changes nothing.
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
        let (from, numbering) = match self.placeable(video, published_at, output_dir) {
            Ok(placeable) => placeable,
            Err(outcome) => return Ok(outcome),
        };
        let number = match numbering {
            Numbering::Recorded(number) => number,
            Numbering::FromPublishTime(published_at) => self
                .video_repository
                .assign_episode_number(&video.id, published_at)?,
        };
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
        let (from, numbering) = match self.placeable(video, published_at, output_dir) {
            Ok(placeable) => placeable,
            Err(outcome) => return Ok(outcome),
        };
        let number = match numbering {
            Numbering::Recorded(number) => number,
            Numbering::FromPublishTime(published_at) => past_planned(
                self.video_repository
                    .next_episode_number(&video.id, published_at)?,
                planned,
            )?,
        };
        let to = RelocatedFiles::new(video, &from, &number).filename;
        Ok(Relocation::Moved { from, to, number })
    }
}

impl VideoRelocator {
    /// The legacy video's recorded file and how it gets its number when it
    /// can be relocated, else the outcome to report instead.
    fn placeable(
        &self,
        video: &Video,
        published_at: Option<DateTime<Utc>>,
        output_dir: &Path,
    ) -> Result<(String, Numbering), Relocation> {
        let Some(from) = video
            .filename
            .clone()
            .filter(|_| video.is_in_legacy_layout())
        else {
            return Err(Relocation::AlreadyInPlace);
        };
        let numbering = match (video.episode_number, published_at) {
            (Some(number), _) => Numbering::Recorded(number),
            (None, Some(published_at)) => Numbering::FromPublishTime(published_at),
            (None, None) => return Err(Relocation::Skipped(SkipReason::NoPublishTimestamp)),
        };
        if !self.video_file_repository.file_exists(output_dir, &from)
            && !self.is_half_moved(video, &from, &numbering, output_dir)
        {
            return Err(Relocation::Skipped(SkipReason::FileMissing));
        }
        Ok((from, numbering))
    }

    /// Whether an earlier run moved the media to its episode path but stopped
    /// before recording it: the number was recorded before any move, so the
    /// target is known and the rest of the set is completed, not re-downloaded.
    fn is_half_moved(
        &self,
        video: &Video,
        from: &str,
        numbering: &Numbering,
        output_dir: &Path,
    ) -> bool {
        match numbering {
            Numbering::Recorded(number) => self.video_file_repository.file_exists(
                output_dir,
                &RelocatedFiles::new(video, from, number).filename,
            ),
            Numbering::FromPublishTime(_) => false,
        }
    }

    fn move_files(
        &self,
        video: &Video,
        files: &RelocatedFiles,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let from = video.filename.iter().chain(&video.thumbnail_filename);
        let to = std::iter::once(&files.filename).chain(&files.thumbnail_filename);
        from.zip(to)
            .filter(|(from, to)| !self.already_moved(from, to, output_dir))
            .try_for_each(|(from, to)| {
                self.video_file_repository
                    .move_file(&output_dir.join(from), &output_dir.join(to))
            })
    }

    /// A file an interrupted earlier run already moved: gone from `from`,
    /// present at `to`.
    fn already_moved(&self, from: &str, to: &str, output_dir: &Path) -> bool {
        !self.video_file_repository.file_exists(output_dir, from)
            && self.video_file_repository.file_exists(output_dir, to)
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
        self.video_metadata_repository.save(
            &video.id,
            &metadata.with_thumb(files.thumb.clone()),
            number,
            &output_dir.join(&files.nfo_filename),
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
/// that is later: what assigning would give once those were recorded. A
/// playlist number is recorded already, so it is never bumped.
fn past_planned(next: EpisodeNumber, planned: &[EpisodeNumber]) -> anyhow::Result<EpisodeNumber> {
    let EpisodeNumber::Channel(next) = next else {
        return Ok(next);
    };
    let last_planned = planned
        .iter()
        .filter_map(|number| match number {
            EpisodeNumber::Channel(number) => Some(number),
            EpisodeNumber::Playlist(_) => None,
        })
        .filter(|number| {
            number.season() == next.season() && number.same_day_prefix() == next.same_day_prefix()
        })
        .map(ChannelEpisodeNumber::episode)
        .max();
    match last_planned {
        Some(last) if last >= next.episode() => {
            Ok(ChannelEpisodeNumber::new(next.season(), last + 1)?.into())
        }
        _ => Ok(next.into()),
    }
}

/// Where a relocated video's episode number comes from: the one already
/// recorded (always, for a playlist video), else one assigned from its
/// publish time.
enum Numbering {
    Recorded(EpisodeNumber),
    FromPublishTime(DateTime<Utc>),
}

/// A legacy video's media, thumbnail and NFO paths in its season folder,
/// relative to the show directory, and the thumbnail's name as the NFO
/// references it.
struct RelocatedFiles {
    filename: String,
    thumbnail_filename: Option<String>,
    nfo_filename: String,
    thumb: Option<String>,
}

impl RelocatedFiles {
    fn new(video: &Video, from: &str, number: &EpisodeNumber) -> Self {
        let extension = Path::new(from)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("mp4");
        let paths = EpisodePaths::new(number, &video.title);
        let has_thumbnail = video.thumbnail_filename.is_some();
        Self {
            filename: paths.media(extension),
            thumbnail_filename: has_thumbnail.then(|| paths.thumbnail()),
            nfo_filename: paths.nfo(),
            thumb: has_thumbnail.then(|| paths.thumb()),
        }
    }
}
