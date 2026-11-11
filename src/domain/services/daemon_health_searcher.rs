use crate::domain::daemon::{DaemonHealth, StepCounts};
use crate::domain::task::{ScheduledTask, TaskStatus};
use crate::domain::video::{StepStatus, Video, VideoStatus};
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::domain_events::event_repository::EventRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Reads what the daemon's heartbeat reports — see the `daemon`
/// capability's Heartbeat Logging.
pub struct DaemonHealthSearcher {
    task_repository: Arc<dyn TaskRepository>,
    event_repository: Arc<dyn EventRepository>,
    video_repository: Arc<dyn VideoRepository>,
    clock: Arc<dyn Clock>,
}

impl DaemonHealthSearcher {
    pub fn new(
        task_repository: Arc<dyn TaskRepository>,
        event_repository: Arc<dyn EventRepository>,
        video_repository: Arc<dyn VideoRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            task_repository,
            event_repository,
            video_repository,
            clock,
        }
    }
}

pub trait DaemonHealthSearcherApi: Send + Sync {
    /// The task and event queues, their dead letters, and the library's
    /// videos by step status, as they are now.
    fn search(&self) -> anyhow::Result<DaemonHealth>;
}

impl DaemonHealthSearcherApi for DaemonHealthSearcher {
    fn search(&self) -> anyhow::Result<DaemonHealth> {
        let tasks = self.task_repository.list_non_completed()?;
        let videos = self.video_repository.list()?;
        Ok(DaemonHealth {
            pending_tasks: Self::pending_per_lane(&tasks),
            running_tasks: Self::count_status(&tasks, TaskStatus::Running),
            oldest_eligible_task_age_seconds: self.oldest_eligible_age(&tasks),
            pending_events: self.event_repository.list_pending()?.len(),
            dead_lettered_tasks: self.task_repository.list_dead_lettered()?.len(),
            dead_lettered_events: self.event_repository.list_dead_lettered()?.len(),
            downloads: Self::step_counts(videos.iter().map(Self::download_status)),
            summaries: Self::step_counts(videos.iter().map(|video| video.summary.status)),
            categorizations: Self::step_counts(
                videos.iter().map(|video| video.categorization.status),
            ),
        })
    }
}

impl DaemonHealthSearcher {
    fn pending_per_lane(tasks: &[ScheduledTask]) -> BTreeMap<&'static str, usize> {
        tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Pending)
            .fold(BTreeMap::new(), |mut lanes, task| {
                *lanes.entry(task.task.lane().as_str()).or_insert(0) += 1;
                lanes
            })
    }

    fn count_status(tasks: &[ScheduledTask], status: TaskStatus) -> usize {
        tasks.iter().filter(|task| task.status == status).count()
    }

    /// How long the pending task that has been due longest has waited.
    fn oldest_eligible_age(&self, tasks: &[ScheduledTask]) -> Option<i64> {
        let now = self.clock.now();
        tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Pending && task.run_at <= now)
            .map(|task| (now - task.run_at).num_seconds())
            .max()
    }

    /// A download's step status; one In Progress counts as none of
    /// pending, retrying or failed.
    fn download_status(video: &Video) -> Option<StepStatus> {
        match video.download.status() {
            VideoStatus::Pending => Some(StepStatus::Pending),
            VideoStatus::Retrying => Some(StepStatus::Retrying),
            VideoStatus::Failed => Some(StepStatus::Failed),
            _ => None,
        }
    }

    fn step_counts(statuses: impl Iterator<Item = impl Into<Option<StepStatus>>>) -> StepCounts {
        statuses.fold(StepCounts::default(), |counts, status| {
            match status.into() {
                Some(StepStatus::Pending) => StepCounts {
                    pending: counts.pending + 1,
                    ..counts
                },
                Some(StepStatus::Retrying) => StepCounts {
                    retrying: counts.retrying + 1,
                    ..counts
                },
                Some(StepStatus::Failed) => StepCounts {
                    failed: counts.failed + 1,
                    ..counts
                },
                _ => counts,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel_video::VideoAddedToChannel;
    use crate::domain::event::DomainEvent;
    use crate::domain::task::{DeadLetteredTask, Task};
    use crate::domain::video::{Video, YoutubeVideoId, video_fixtures::downloaded_video};
    use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::{
        EventPublisher, SqliteEventPublisher,
    };
    use crate::infrastructure::shared::domain_events::event_repository::SqliteEventRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Duration, Utc};

    #[test]
    fn it_should_report_queue_and_library_counts() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(now())),
        ));
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let event_repository = Arc::new(SqliteEventRepository::new(
            db.database(),
            Arc::new(FixedClock(now())),
        ));
        task_repository
            .schedule(&download("a"), now() - Duration::minutes(10))
            .unwrap();
        task_repository
            .schedule(&download("b"), now() - Duration::minutes(5))
            .unwrap();
        task_repository
            .schedule(&sync("c"), now() + Duration::hours(1))
            .unwrap();
        task_repository
            .schedule(&thumbnail("d"), now() - Duration::minutes(1))
            .unwrap();
        task_repository.claim(4, now()).unwrap();
        task_repository.schedule(&Task::UpdateYtdlp, now()).unwrap();
        task_repository
            .dead_letter(&DeadLetteredTask {
                original_task_id: 5,
                task: Task::UpdateYtdlp,
                retries: 3,
                last_error: "boom".to_string(),
                created_at: now(),
                failed_at: now(),
            })
            .unwrap();
        SqliteEventPublisher::new(db.database(), Arc::new(FixedClock(now())))
            .publish(&DomainEvent::VideoAddedToChannel(VideoAddedToChannel {
                channel_id: "@somechannel".to_string(),
                video_id: "a".to_string(),
            }))
            .unwrap();
        store(
            &video_repository,
            &Video::create(YoutubeVideoId::new("yt1").unwrap(), "One", now())
                .mark_errored("HTTP Error 503", now()),
        );
        store(
            &video_repository,
            &downloaded_video("yt2", "Two.mp4", now()).mark_summary_failed(now()),
        );
        let searcher = DaemonHealthSearcher::new(
            task_repository,
            event_repository,
            video_repository,
            Arc::new(FixedClock(now())),
        );

        let health = searcher.search();

        assert_eq!(
            health.map_err(|e| e.to_string()),
            Ok(DaemonHealth {
                pending_tasks: BTreeMap::from([("download", 2), ("video_sync", 1)]),
                running_tasks: 1,
                oldest_eligible_task_age_seconds: Some(600),
                pending_events: 1,
                dead_lettered_tasks: 1,
                dead_lettered_events: 0,
                downloads: StepCounts {
                    pending: 0,
                    retrying: 1,
                    failed: 0,
                },
                summaries: StepCounts {
                    pending: 1,
                    retrying: 0,
                    failed: 1,
                },
                categorizations: StepCounts {
                    pending: 2,
                    retrying: 0,
                    failed: 0,
                },
            })
        );
    }

    #[test]
    fn it_should_report_no_oldest_task_if_none_is_eligible() {
        let db = TestDatabase::new();
        let task_repository = Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(now())),
        ));
        task_repository
            .schedule(&sync("c"), now() + Duration::hours(1))
            .unwrap();
        let searcher = DaemonHealthSearcher::new(
            task_repository,
            Arc::new(SqliteEventRepository::new(
                db.database(),
                Arc::new(FixedClock(now())),
            )),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(FixedClock(now())),
        );

        let health = searcher.search();

        assert_eq!(
            health.map_err(|e| e.to_string()),
            Ok(DaemonHealth {
                pending_tasks: BTreeMap::from([("video_sync", 1)]),
                oldest_eligible_task_age_seconds: None,
                ..DaemonHealth::default()
            })
        );
    }

    fn store(video_repository: &SqliteVideoRepository, video: &Video) {
        video_repository.insert(video).unwrap();
        video_repository.update(video).unwrap();
    }

    fn download(video_id: &str) -> Task {
        Task::DownloadVideo {
            video_id: video_id.to_string(),
        }
    }

    fn sync(video_id: &str) -> Task {
        Task::SyncVideo {
            video_id: video_id.to_string(),
        }
    }

    fn thumbnail(video_id: &str) -> Task {
        Task::FetchThumbnail {
            video_id: video_id.to_string(),
        }
    }

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }
}
