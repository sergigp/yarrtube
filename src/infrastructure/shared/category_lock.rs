use std::sync::{Mutex, MutexGuard, PoisonError};

/// In-process lock serializing the writes of video categories, so a source's
/// categories read by one writer can't be outdated by another before it
/// saves (a reconcile pass against a members change, a sync against a
/// category delete). Not persisted: nothing it guards spans a restart.
#[derive(Default)]
pub struct CategoryLock {
    held: Mutex<()>,
}

impl CategoryLock {
    /// Locks, waiting for whoever holds it; released when the guard drops.
    pub fn lock(&self) -> MutexGuard<'_, ()> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
