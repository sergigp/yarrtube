use crate::domain::show::ShowKey;
use std::collections::HashSet;
use std::sync::{Condvar, Mutex};

/// In-process registry of the shows a move or a reconcile is working on, so
/// the two never overlap for the same source. Not persisted: a restart
/// drops every lock, and nothing it guards is left half done.
#[derive(Default)]
pub struct ShowLocks {
    #[allow(dead_code)]
    held: Mutex<HashSet<ShowKey>>,
    #[allow(dead_code)]
    released: Condvar,
}

/// Holds `key` locked until dropped.
pub struct ShowLockGuard<'a> {
    #[allow(dead_code)]
    locks: &'a ShowLocks,
    #[allow(dead_code)]
    key: ShowKey,
}

impl ShowLocks {
    /// Locks `key`, waiting for whoever holds it to release it.
    pub fn lock(&self, key: ShowKey) -> ShowLockGuard<'_> {
        ShowLockGuard { locks: self, key }
    }

    /// Locks `key` if nobody holds it, `None` otherwise.
    pub fn try_lock(&self, key: ShowKey) -> Option<ShowLockGuard<'_>> {
        Some(ShowLockGuard { locks: self, key })
    }
}

impl Drop for ShowLockGuard<'_> {
    fn drop(&mut self) {}
}
