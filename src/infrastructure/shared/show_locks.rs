use crate::domain::show::ShowKey;
use std::collections::HashSet;
use std::sync::{Condvar, Mutex};

/// In-process registry of the shows a move or a reconcile is working on, so
/// the two never overlap for the same source. Not persisted: a restart
/// drops every lock, and nothing it guards is left half done.
#[derive(Default)]
pub struct ShowLocks {
    held: Mutex<HashSet<ShowKey>>,
    released: Condvar,
}

/// Holds `key` locked until dropped.
pub struct ShowLockGuard<'a> {
    locks: &'a ShowLocks,
    key: ShowKey,
}

impl ShowLocks {
    /// Locks `key`, waiting for whoever holds it to release it.
    pub fn lock(&self, key: ShowKey) -> ShowLockGuard<'_> {
        let held = self.held.lock().unwrap();
        let mut held = self
            .released
            .wait_while(held, |held| held.contains(&key))
            .unwrap();
        held.insert(key.clone());
        ShowLockGuard { locks: self, key }
    }

    /// Locks `key` if nobody holds it, `None` otherwise.
    pub fn try_lock(&self, key: ShowKey) -> Option<ShowLockGuard<'_>> {
        let locked = self.held.lock().unwrap().insert(key.clone());
        locked.then(|| ShowLockGuard { locks: self, key })
    }
}

impl Drop for ShowLockGuard<'_> {
    fn drop(&mut self) {
        self.locks.held.lock().unwrap().remove(&self.key);
        self.locks.released.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::ChannelHandle;
    use crate::domain::playlist::PlaylistId;

    #[test]
    fn it_should_refuse_try_lock_while_held() {
        let locks = ShowLocks::default();
        let _held = locks.lock(playlist("PL1"));

        let second = locks.try_lock(playlist("PL1"));

        assert!(second.is_none());
    }

    #[test]
    fn it_should_release_on_drop() {
        let locks = ShowLocks::default();
        drop(locks.lock(playlist("PL1")));

        let again = locks.try_lock(playlist("PL1"));

        assert!(again.is_some());
    }

    #[test]
    fn it_should_block_lock_until_released() {
        let locks = ShowLocks::default();
        let held = locks.lock(playlist("PL1"));

        let (waited, acquired) = std::thread::scope(|scope| {
            let waiting = scope.spawn(|| drop(locks.lock(playlist("PL1"))));
            std::thread::sleep(std::time::Duration::from_millis(100));
            let waited = !waiting.is_finished();
            drop(held);
            (waited, waiting.join().is_ok())
        });

        assert_eq!((waited, acquired), (true, true));
    }

    #[test]
    fn it_should_not_contend_across_keys() {
        let locks = ShowLocks::default();
        let _held = locks.lock(playlist("PL1"));

        let others = [
            locks.try_lock(playlist("PL2")).is_some(),
            locks
                .try_lock(ShowKey::Channel(
                    ChannelHandle::new("@somechannel").unwrap(),
                ))
                .is_some(),
        ];

        assert_eq!(others, [true, true]);
    }

    fn playlist(id: &str) -> ShowKey {
        ShowKey::Playlist(PlaylistId::new(id).unwrap())
    }
}
