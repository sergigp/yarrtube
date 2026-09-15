use crate::domain::channel::ChannelHandle;
use crate::domain::services::ChannelVideoReconciler;
use crate::infrastructure::repositories::event_subscriber::EventSubscriber;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ChannelCreatedPayload {
    channel_id: String,
}

/// Reacts to `ChannelCreated` by running one reconcile pass for the new
/// channel, as that event's own processing (not as a separately scheduled
/// task) — mirrors `ReconcileOnPlaylistCreated`.
pub struct ReconcileOnChannelCreated {
    channel_video_reconciler: ChannelVideoReconciler,
}

impl ReconcileOnChannelCreated {
    pub fn new(channel_video_reconciler: ChannelVideoReconciler) -> Self {
        Self {
            channel_video_reconciler,
        }
    }
}

impl EventSubscriber for ReconcileOnChannelCreated {
    fn handle(&self, payload: &str) -> anyhow::Result<()> {
        let payload: ChannelCreatedPayload = serde_json::from_str(payload)?;
        let Ok(channel_id) = ChannelHandle::new(payload.channel_id) else {
            return Ok(());
        };
        self.channel_video_reconciler.reconcile(channel_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, VideoLimit};
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::shared::Quality;
    use crate::domain::task::Task;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, FakeChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::FakeChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_channel_videos_repository::FakeChannelVideosRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(0, 0).unwrap()
    }

    fn reconciler(
        channel_repository: Arc<dyn ChannelRepository>,
    ) -> (ChannelVideoReconciler, Arc<FakeTaskRepository>) {
        let task_repository = Arc::new(FakeTaskRepository::default());
        let reconciler = ChannelVideoReconciler::new(
            channel_repository,
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakeChannelVideoRepository::default()),
            Arc::new(FakeChannelVideosRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            task_repository.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );
        (reconciler, task_repository)
    }

    #[test]
    fn it_should_no_op_when_the_payload_channel_id_is_invalid() {
        let (reconciler, task_repository) = reconciler(Arc::new(FakeChannelRepository::default()));
        let subscriber = ReconcileOnChannelCreated::new(reconciler);

        subscriber.handle(r#"{"channel_id": ""}"#).unwrap();

        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_reconcile_the_channel_named_in_the_payload() {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        channel_repository
            .insert(&Channel::create(
                ChannelHandle::new("@somechannel").unwrap(),
                "Some Channel",
                "UC123",
                Quality::High,
                VideoLimit::new(10).unwrap(),
                PlaylistPath::new("creators/somechannel").unwrap(),
                fixed_timestamp(),
            ))
            .unwrap();
        let (reconciler, task_repository) = reconciler(channel_repository);
        let subscriber = ReconcileOnChannelCreated::new(reconciler);

        subscriber
            .handle(r#"{"channel_id": "@somechannel"}"#)
            .unwrap();

        assert_eq!(
            task_repository
                .scheduled
                .lock()
                .unwrap()
                .iter()
                .map(|(task, _run_at)| task.clone())
                .collect::<Vec<_>>(),
            vec![Task::ReconcileChannel {
                channel_id: "@somechannel".to_string()
            }]
        );
    }

    #[test]
    fn it_should_no_op_when_the_channel_no_longer_exists() {
        let (reconciler, task_repository) = reconciler(Arc::new(FakeChannelRepository::default()));
        let subscriber = ReconcileOnChannelCreated::new(reconciler);

        let result = subscriber.handle(r#"{"channel_id": "@missing"}"#);

        assert!(result.is_ok());
        assert!(task_repository.scheduled.lock().unwrap().is_empty());
    }
}
