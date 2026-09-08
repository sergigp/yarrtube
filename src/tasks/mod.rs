pub mod sync_playlist_task;

use crate::domain::video::VideoService;
use crate::infrastructure::repositories::task_executor::HandlerRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use sync_playlist_task::SyncPlaylistTask;

/// Maps each task type to the single handler that runs it, handed to the
/// task executor at composition time.
pub fn registry(video_service: VideoService) -> HandlerRegistry {
    let mut registry: HandlerRegistry = HashMap::new();
    registry.insert(
        "sync_playlist".to_string(),
        Arc::new(SyncPlaylistTask::new(video_service)),
    );
    registry
}
