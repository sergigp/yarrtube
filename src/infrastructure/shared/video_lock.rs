use crate::domain::video::VideoRecordId;
use std::collections::HashSet;
use std::sync::{Condvar, Mutex, PoisonError};

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
        let held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        let mut held = self
            .released
            .wait_while(held, |held| held.contains(video_id))
            .unwrap_or_else(PoisonError::into_inner);
        held.insert(video_id.clone());
        VideoLockGuard {
            lock: self,
            video_id: video_id.clone(),
        }
    }
}

impl Drop for VideoLockGuard<'_> {
    fn drop(&mut self) {
        self.lock
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.video_id);
        self.lock.released.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn it_should_block_a_second_lock_of_the_same_video_until_released() {
        let lock = VideoLock::default();
        let held = lock.lock(&video_id("v1"));
        let (acquired_tx, acquired) = mpsc::channel();

        let (blocked_while_held, acquired_once_released) = std::thread::scope(|scope| {
            scope.spawn(|| {
                let _guard = lock.lock(&video_id("v1"));
                acquired_tx.send(()).unwrap();
            });
            let blocked_while_held = acquired.recv_timeout(Duration::from_millis(100)).is_err();
            drop(held);
            (blocked_while_held, acquired.recv_timeout(TIMEOUT).is_ok())
        });

        assert_eq!((blocked_while_held, acquired_once_released), (true, true));
    }

    #[test]
    fn it_should_not_block_locks_of_different_videos() {
        let lock = VideoLock::default();
        let held = lock.lock(&video_id("v1"));
        let (acquired_tx, acquired) = mpsc::channel();

        let acquired_while_held = std::thread::scope(|scope| {
            scope.spawn(|| {
                let _guard = lock.lock(&video_id("v2"));
                acquired_tx.send(()).unwrap();
            });
            let acquired_while_held = acquired.recv_timeout(TIMEOUT).is_ok();
            drop(held);
            acquired_while_held
        });

        assert!(acquired_while_held);
    }

    /// How long a lock that should be acquired gets: generous, so a loaded
    /// machine never fails the test, and only waited out on a regression.
    const TIMEOUT: Duration = Duration::from_secs(5);

    fn video_id(id: &str) -> VideoRecordId {
        VideoRecordId::new(id).unwrap()
    }
}
