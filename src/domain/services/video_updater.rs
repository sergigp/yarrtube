use crate::domain::video::{StaleVideo, Video, VideoId};
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use anyhow::anyhow;
use std::sync::Arc;

/// How many times an update is applied on top of a newer stored video
/// before giving up.
pub const MAX_UPDATE_ATTEMPTS: usize = 5;

/// The one way to change a stored video — see the `video-write-consistency`
/// capability.
#[derive(Clone)]
pub struct VideoUpdater {
    video_repository: Arc<dyn VideoRepository>,
}

impl VideoUpdater {
    pub fn new(video_repository: Arc<dyn VideoRepository>) -> Self {
        Self { video_repository }
    }
}

pub trait VideoUpdaterApi: Send + Sync {
    /// Applies `transition` to the stored video `id` and stores the result,
    /// re-reading it and applying `transition` again whenever another writer
    /// stored it first, so neither change is lost. `transition` must have no
    /// side effects: it may run more than once. `Ok(None)` when the video is
    /// gone or `transition` declines (returns `None`); otherwise the video as
    /// stored.
    fn update<F>(&self, id: &VideoId, transition: F) -> anyhow::Result<Option<Video>>
    where
        F: Fn(Video) -> Option<Video>;
}

impl VideoUpdaterApi for VideoUpdater {
    fn update<F>(&self, id: &VideoId, transition: F) -> anyhow::Result<Option<Video>>
    where
        F: Fn(Video) -> Option<Video>,
    {
        for _ in 0..MAX_UPDATE_ATTEMPTS {
            let Some(updated) = self.video_repository.find(id)?.and_then(&transition) else {
                return Ok(None);
            };
            match self.video_repository.update(&updated) {
                Ok(()) => return Ok(Some(updated.next_version())),
                Err(e) if e.is::<StaleVideo>() => continue,
                Err(e) => return Err(e),
            }
        }
        Err(anyhow!(
            "video {id} kept changing: gave up after {MAX_UPDATE_ATTEMPTS} attempts"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video::YoutubeVideoId;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use chrono::{DateTime, Utc};
    use std::cell::Cell;

    #[test]
    fn it_should_apply_the_transition_on_top_of_a_concurrent_write() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let other_writer = SqliteVideoRepository::new(db.database());
        let video = my_video();
        video_repository.insert(&video).unwrap();
        let updater = VideoUpdater::new(video_repository.clone());
        let first_call = Cell::new(true);

        let result = updater.update(&video.id, |read| {
            if first_call.replace(false) {
                let stored = other_writer.find(&video.id).unwrap().unwrap();
                other_writer
                    .update(&stored.with_title("Renamed", later()))
                    .unwrap();
            }
            Some(read.mark_watched(later()))
        });

        let expected = Video {
            version: 2,
            ..video.with_title("Renamed", later()).mark_watched(later())
        };
        assert_eq!(
            result.map_err(|e| e.to_string()),
            Ok(Some(expected.clone()))
        );
        assert_eq!(video_repository.list().unwrap(), vec![expected]);
    }

    #[test]
    fn it_should_skip_if_the_video_is_gone() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let updater = VideoUpdater::new(video_repository.clone());

        let result = updater.update(&my_video().id, |read| Some(read.mark_watched(later())));

        assert_eq!(result.map_err(|e| e.to_string()), Ok(None));
        assert_eq!(video_repository.list().unwrap(), vec![]);
    }

    #[test]
    fn it_should_skip_if_the_transition_declines() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        let updater = VideoUpdater::new(video_repository.clone());

        let result = updater.update(&video.id, |_| None);

        assert_eq!(result.map_err(|e| e.to_string()), Ok(None));
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    #[test]
    fn it_should_fail_if_the_video_keeps_changing() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let other_writer = SqliteVideoRepository::new(db.database());
        let video = my_video();
        video_repository.insert(&video).unwrap();
        let updater = VideoUpdater::new(video_repository.clone());

        let result = updater.update(&video.id, |read| {
            let stored = other_writer.find(&video.id).unwrap().unwrap();
            other_writer.update(&stored).unwrap();
            Some(read.mark_watched(later()))
        });

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(format!(
                "video {} kept changing: gave up after 5 attempts",
                video.id
            ))
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![Video {
                version: 5,
                ..video
            }]
        );
    }

    #[test]
    fn it_should_fail_at_once_if_the_update_fails() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video = my_video();
        video_repository.insert(&video).unwrap();
        poison_the_write_connection(&db);
        let updater = VideoUpdater::new(video_repository.clone());

        let result = updater.update(&video.id, |read| Some(read.mark_watched(later())));

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err("database lock poisoned".to_string())
        );
        assert_eq!(video_repository.list().unwrap(), vec![video]);
    }

    fn my_video() -> Video {
        Video::create(YoutubeVideoId::new("yt1").unwrap(), "My Video", now())
    }

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn later() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_060, 0).unwrap()
    }

    /// A writer panicking mid-operation: every later write fails with a
    /// non-stale error, while reads keep working on their own connection.
    fn poison_the_write_connection(db: &TestDatabase) {
        let database = db.database();
        let _ = std::thread::spawn(move || {
            let _write = database.write().unwrap();
            panic!("a writer panicked mid-operation");
        })
        .join();
    }
}
