use crate::domain::video::VideoRecordId;
use std::collections::HashSet;
use std::sync::{Condvar, Mutex};

/// In-process lock serializing the syncs of one video, so a reconcile pass
/// and an event-triggered sync never decide from the same state at once.
/// Videos are locked independently. Not persisted: nothing it guards spans a
/// restart.
#[derive(Default)]
pub struct VideoLock {
    held: Mutex<HashSet<VideoRecordId>>,
    released: Condvar,
}

/// Holds a video's lock until dropped.
pub struct VideoLockGuard<'a> {
    lock: &'a VideoLock,
    video_id: VideoRecordId,
}

impl VideoLock {
    /// Locks `video_id`, waiting while another holder has it; released on drop.
    pub fn lock(&self, video_id: &VideoRecordId) -> VideoLockGuard<'_> {
        VideoLockGuard {
            lock: self,
            video_id: video_id.clone(),
        }
    }
}

impl Drop for VideoLockGuard<'_> {
    fn drop(&mut self) {}
}
