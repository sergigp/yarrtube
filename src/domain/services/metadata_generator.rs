use crate::domain::video::Video;
use crate::domain::video_metadata::{VideoMetadata, build_video_metadata, resolve_sorttitle};
use crate::infrastructure::repositories::youtube_metadata_repository::YoutubeMetadataRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;
use tracing::warn;

/// Builds a video's `movie.nfo` metadata from YouTube, the one place the
/// `sorttitle` rule is applied. Shared by `VideoDownloader` (at download
/// time) and `InternalVideoReconciler` (repairing missing metadata).
pub struct MetadataGenerator {
    youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
    clock: Arc<dyn Clock>,
}

impl MetadataGenerator {
    pub fn new(
        youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            youtube_metadata_repository,
            clock,
        }
    }
}

pub trait MetadataGeneratorApi: Send + Sync {
    /// Fetches `video`'s YouTube metadata and builds its `VideoMetadata`,
    /// resolving `sorttitle` from `sort_position` (its playlist position),
    /// or from the publish date when `None`, and referencing `thumbnail`.
    /// `None` (logged) when YouTube has no metadata for the video or the
    /// fetch fails: metadata generation never fails its caller, and the
    /// next reconcile pass tries again.
    fn generate(
        &self,
        video: &Video,
        sort_position: Option<i64>,
        thumbnail: Option<String>,
    ) -> Option<VideoMetadata>;
}

impl MetadataGeneratorApi for MetadataGenerator {
    fn generate(
        &self,
        video: &Video,
        sort_position: Option<i64>,
        thumbnail: Option<String>,
    ) -> Option<VideoMetadata> {
        let metadata = match self.youtube_metadata_repository.find(&video.youtube_id) {
            Ok(Some(metadata)) => metadata,
            Ok(None) => {
                warn!(video_id = %video.id, "no YouTube metadata found for video, skipping metadata generation");
                return None;
            }
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch YouTube metadata, skipping metadata generation");
                return None;
            }
        };

        let sorttitle = resolve_sorttitle(&metadata.title, metadata.published_at, sort_position);
        Some(build_video_metadata(
            &video.youtube_id,
            &metadata,
            sorttitle,
            thumbnail,
            self.clock.now(),
        ))
    }
}
