pub mod sync_playlist_task;

use crate::domain::playlist::PlaylistService;
use crate::infrastructure::repositories::task_executor::HandlerRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use sync_playlist_task::SyncPlaylistTask;

/// Maps each task type to the single handler that runs it, handed to the
/// task executor at composition time.
pub fn registry(playlist_service: PlaylistService) -> HandlerRegistry {
    let mut registry: HandlerRegistry = HashMap::new();
    registry.insert(
        "sync_playlist".to_string(),
        Arc::new(SyncPlaylistTask::new(playlist_service)),
    );
    registry
}
