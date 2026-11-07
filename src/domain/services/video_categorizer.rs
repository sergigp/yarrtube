use crate::domain::category::Category;
use crate::domain::services::{VideoCategoryAssigner, VideoOwnerFinder};
use crate::domain::video::{Video, VideoOwner};
use crate::infrastructure::repositories::ollama_llm_server_repository::LlmServerRepository;
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_preferences_repository::PreferencesRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_summary_repository::VideoSummaryRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Asks the LLM server which categories a video fits — see the
/// `video-categorization` capability. Runs from the `categorize_video` task;
/// `VideoReconciler` decides when to schedule it.
#[derive(Clone)]
#[allow(dead_code)]
pub struct VideoCategorizer {
    preferences_repository: Arc<dyn PreferencesRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    category_repository: Arc<dyn CategoryRepository>,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    video_summary_repository: Arc<dyn VideoSummaryRepository>,
    llm_server_repository: Arc<dyn LlmServerRepository>,
    video_category_assigner: VideoCategoryAssigner,
    clock: Arc<dyn Clock>,
}

impl VideoCategorizer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        preferences_repository: Arc<dyn PreferencesRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        category_repository: Arc<dyn CategoryRepository>,
        video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
        video_summary_repository: Arc<dyn VideoSummaryRepository>,
        llm_server_repository: Arc<dyn LlmServerRepository>,
        video_category_assigner: VideoCategoryAssigner,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            preferences_repository,
            video_owner_finder,
            category_repository,
            video_category_verdict_repository,
            video_summary_repository,
            llm_server_repository,
            video_category_assigner,
            clock,
        }
    }
}

pub trait VideoCategorizerApi: Send + Sync {
    /// The categories pending for `video` now: with an AI prompt, not given
    /// by `owner`'s source, without a verdict for it.
    fn pending(&self, video: &Video, owner: &VideoOwner) -> anyhow::Result<Vec<Category>>;
    /// Asks the model about each category pending when it runs, storing
    /// each verdict as it comes. Err when any call was rejected, after
    /// trying all; on the last attempt those become unavailable. Holds no
    /// `VideoLock`: calls take minutes.
    fn categorize(&self, video: &Video, is_last_attempt: bool) -> anyhow::Result<()>;
}

impl VideoCategorizerApi for VideoCategorizer {
    fn pending(&self, _video: &Video, _owner: &VideoOwner) -> anyhow::Result<Vec<Category>> {
        Ok(Vec::new())
    }

    fn categorize(&self, _video: &Video, _is_last_attempt: bool) -> anyhow::Result<()> {
        Ok(())
    }
}
