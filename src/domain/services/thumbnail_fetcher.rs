use crate::domain::shared::VideoRecordId;
use crate::domain::video::Video;
use crate::domain::video::VideoStatus;
use crate::domain::video::top_level_entry;
use crate::domain::video::video_filename::VideoFilename;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tracing::warn;

/// Best-effort thumbnail fetch, independent of and ahead of a video's full
/// download — see the `video-thumbnails` capability. Called from every
/// video-creation site and from each reconciler's missing-thumbnail
/// recovery pass; never returns an error to its caller, so no call site
/// needs its own try/catch-and-ignore boilerplate.
#[derive(Clone)]
pub struct ThumbnailFetcher {
    video_repository: Arc<dyn VideoRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    clock: Arc<dyn Clock>,
}

impl ThumbnailFetcher {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            video_repository,
            video_downloader_repository,
            clock,
        }
    }

    /// No-ops if `video` already has a recorded thumbnail. Otherwise fetches
    /// one into `output_dir` and persists it via `Video::with_thumbnail` on
    /// success; any failure (a clean "no thumbnail available" outcome, or a
    /// systemic error) is logged and swallowed, leaving `video` untouched.
    /// A `Downloaded` video already has its own folder recorded via
    /// `filename` (e.g. a missing-thumbnail recovery pass running against a
    /// video whose full download already ran) — that folder is reused
    /// verbatim instead of resolving a fresh, collision-suffixed one.
    pub fn fetch(&self, video: &Video, output_dir: &Path) {
        if video.thumbnail_filename.is_some() {
            return;
        }

        let existing_folder = video.filename.as_deref().map(top_level_entry);
        let filename = VideoFilename::from_title(&video.title);
        let fetched = self.video_downloader_repository.fetch_thumbnail(
            &video.youtube_id.to_url(),
            filename.as_str(),
            video.youtube_id.as_str(),
            output_dir,
            existing_folder,
        );

        match fetched {
            Ok(Some(fetched)) => {
                let thumbnail_filename = format!("{}/{}", fetched.folder, fetched.filename);
                let updated = video
                    .clone()
                    .with_thumbnail(thumbnail_filename, self.clock.now());
                if let Err(e) = self.video_repository.update(&updated) {
                    warn!(video_id = %video.id, error = %e, "failed to persist fetched thumbnail");
                }
            }
            Ok(None) => {
                warn!(video_id = %video.id, "no thumbnail available for video");
            }
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch video thumbnail");
            }
        }
    }

    /// Missing-thumbnail recovery pass over `videos`, shared by both
    /// reconcilers' `reconcile_filesystem`. Skips a video with no
    /// thumbnail if it's in `skip_ids` (just reset for redownload this same
    /// pass — see `fetch`'s own reasons this must not run for it) or if its
    /// real download is currently `InProgress` (a concurrent `DownloadVideo`
    /// task owns its not-yet-recorded output folder; fetching now would
    /// resolve `existing_folder` to `None` and collide with it, spawning a
    /// stray sibling folder that then gets permanently protected from the
    /// orphan sweep).
    pub fn fetch_missing(
        &self,
        videos: &[Video],
        skip_ids: &HashSet<&VideoRecordId>,
        output_dir: &Path,
    ) {
        for video in videos.iter().filter(|v| {
            v.thumbnail_filename.is_none()
                && !skip_ids.contains(&v.id)
                && v.status != VideoStatus::InProgress
        }) {
            self.fetch(video, output_dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::shared::VideoId;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use crate::infrastructure::shared::ytdlp::FetchedThumbnail;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn video() -> Video {
        Video::create(VideoId::new("vid1").unwrap(), "My Video", fixed_timestamp())
    }

    fn fetcher(
        video_repository: Arc<FakeVideoRepository>,
        downloader: Arc<FakeVideoDownloaderRepository>,
    ) -> ThumbnailFetcher {
        ThumbnailFetcher::new(
            video_repository,
            downloader,
            Arc::new(FixedClock(fixed_timestamp())),
        )
    }

    #[test]
    fn it_should_not_fetch_when_the_video_already_has_a_thumbnail() {
        let video = video().with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::default());
        let fetcher = fetcher(video_repository, downloader.clone());

        fetcher.fetch(&video, Path::new("/videos/my-playlist"));

        assert_eq!(downloader.thumbnail_calls_count(), 0);
    }

    #[test]
    fn it_should_persist_the_fetched_thumbnail_on_success() {
        let video = video();
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(
            FakeVideoDownloaderRepository::default().with_thumbnail_result(Some(
                FetchedThumbnail {
                    folder: "My Video".to_string(),
                    filename: "My Video.jpg".to_string(),
                },
            )),
        );
        let fetcher = fetcher(video_repository.clone(), downloader);

        fetcher.fetch(&video, Path::new("/videos/my-playlist"));

        let updated = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(
            updated.thumbnail_filename,
            Some("My Video/My Video.jpg".to_string())
        );
        assert_eq!(updated.updated_at, fixed_timestamp());
    }

    #[test]
    fn it_should_leave_the_video_untouched_on_a_clean_failure() {
        let video = video();
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository.save(&video).unwrap();
        let downloader =
            Arc::new(FakeVideoDownloaderRepository::default().with_thumbnail_result(None));
        let fetcher = fetcher(video_repository.clone(), downloader);

        fetcher.fetch(&video, Path::new("/videos/my-playlist"));

        let unchanged = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(unchanged, video);
    }

    #[test]
    fn it_should_leave_the_video_untouched_on_a_port_error() {
        let video = video();
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::default().with_thumbnail_error());
        let fetcher = fetcher(video_repository.clone(), downloader);

        fetcher.fetch(&video, Path::new("/videos/my-playlist"));

        let unchanged = video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(unchanged, video);
    }

    #[test]
    fn it_should_reuse_a_downloaded_videos_folder_for_a_missing_thumbnail_recovery_fetch() {
        use crate::domain::shared::Quality;

        let video = video().mark_downloaded(
            Quality::High,
            "My Video/My Video.mp4",
            None,
            None,
            fixed_timestamp(),
        );
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(
            FakeVideoDownloaderRepository::default().with_thumbnail_result(Some(
                FetchedThumbnail {
                    folder: "My Video".to_string(),
                    filename: "My Video.jpg".to_string(),
                },
            )),
        );
        let fetcher = fetcher(video_repository.clone(), downloader.clone());

        fetcher.fetch(&video, Path::new("/videos/my-playlist"));

        let calls = downloader.thumbnail_calls.lock().unwrap();
        assert_eq!(calls[0].4, Some("My Video".to_string()));
    }

    #[test]
    fn it_should_pass_no_existing_folder_for_a_video_with_no_recorded_filename() {
        let video = video();
        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository.save(&video).unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::default());
        let fetcher = fetcher(video_repository.clone(), downloader.clone());

        fetcher.fetch(&video, Path::new("/videos/my-playlist"));

        let calls = downloader.thumbnail_calls.lock().unwrap();
        assert_eq!(calls[0].4, None);
    }
}
