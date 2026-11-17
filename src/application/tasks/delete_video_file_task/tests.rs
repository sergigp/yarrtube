use super::*;
use crate::domain::event::{DomainEvent, ScheduledEvent};
use crate::domain::video::VideoFileDeleted;
use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
use crate::infrastructure::shared::domain_events::event_publisher::{
    EventPublisher, SqliteEventPublisher,
};
use crate::infrastructure::shared::domain_events::event_repository::{
    EventRepository, SqliteEventRepository,
};
use crate::infrastructure::shared::sqlite_connection::TestDatabase;
use crate::infrastructure::shared::system_clock::FixedClock;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn it_should_delete_the_video_file() {
    let db = TestDatabase::new();
    let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
    let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
        video_file_repository.clone(),
        event_publisher(&db),
        "/videos",
    ));

    let result = run(&task, &task_for(Some("My Video.mp4"), None));

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        *video_file_repository.deleted_calls.lock().unwrap(),
        vec![deleted("My Video.mp4")]
    );
}

#[test]
fn it_should_skip_if_video_has_no_filename() {
    let db = TestDatabase::new();
    let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
    let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
        video_file_repository.clone(),
        event_publisher(&db),
        "/videos",
    ));

    let result = run(&task, &task_for(None, None));

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
}

#[test]
fn it_should_fail_if_handed_another_task() {
    let db = TestDatabase::new();
    let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
    let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
        video_file_repository.clone(),
        event_publisher(&db),
        "/videos",
    ));

    let result = run(&task, &Task::UpdateYtdlp);

    assert_eq!(
        result,
        Err("unexpected update_ytdlp task for this handler".to_string())
    );
    assert_eq!(*video_file_repository.deleted_calls.lock().unwrap(), vec![]);
}

#[test]
fn it_should_skip_if_video_file_not_found() {
    let db = TestDatabase::new();
    let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(false)));
    let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
        video_file_repository.clone(),
        event_publisher(&db),
        "/videos",
    ));

    let result = run(&task, &task_for(Some("My Video.mp4"), None));

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        *video_file_repository.deleted_calls.lock().unwrap(),
        vec![deleted("My Video.mp4")]
    );
}

#[test]
fn it_should_delete_the_video_and_thumbnail_files() {
    let db = TestDatabase::new();
    let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
    let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
        video_file_repository.clone(),
        event_publisher(&db),
        "/videos",
    ));

    let result = run(&task, &task_for(Some("My Video.mp4"), Some("My Video.jpg")));

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        *video_file_repository.deleted_calls.lock().unwrap(),
        vec![deleted("My Video.mp4"), deleted("My Video.jpg")]
    );
}

#[test]
fn it_should_skip_thumbnail_if_none_recorded() {
    let db = TestDatabase::new();
    let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
    let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
        video_file_repository.clone(),
        event_publisher(&db),
        "/videos",
    ));

    let result = run(&task, &task_for(Some("My Video.mp4"), None));

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        *video_file_repository.deleted_calls.lock().unwrap(),
        vec![deleted("My Video.mp4")]
    );
}

#[test]
fn it_should_skip_thumbnail_if_file_not_found() {
    let db = TestDatabase::new();
    let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(false)));
    let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
        video_file_repository.clone(),
        event_publisher(&db),
        "/videos",
    ));

    let result = run(&task, &task_for(None, Some("My Video.jpg")));

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        *video_file_repository.deleted_calls.lock().unwrap(),
        vec![deleted("My Video.jpg")]
    );
}

#[test]
fn it_should_delete_an_episodes_media_thumbnail_and_nfo() {
    let db = TestDatabase::new();
    let video_file_repository = Arc::new(FakeVideoFileRepository::default());
    let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
        video_file_repository.clone(),
        event_publisher(&db),
        "/videos",
    ));

    let result = run(
        &task,
        &task_for(
            Some("Season 2026/S2026E031501 - My Video.mp4"),
            Some("Season 2026/S2026E031501 - My Video.jpg"),
        ),
    );

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        *video_file_repository.deleted_calls.lock().unwrap(),
        vec![
            deleted("Season 2026/S2026E031501 - My Video.mp4"),
            deleted("Season 2026/S2026E031501 - My Video.nfo"),
            deleted("Season 2026/S2026E031501 - My Video.jpg"),
        ]
    );
}

#[test]
fn it_should_delete_an_episodes_subtitle_file() {
    let db = TestDatabase::new();
    let video_file_repository = Arc::new(FakeVideoFileRepository::default());
    let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
        video_file_repository.clone(),
        event_publisher(&db),
        "/videos",
    ));

    let result = run(
        &task,
        &Task::DeleteVideoFile {
            filename: Some("Season 2026/S2026E031501 - My Video.mp4".to_string()),
            thumbnail_filename: None,
            subtitle_filename: Some("Season 2026/S2026E031501 - My Video.en.srt".to_string()),
            output_dir: "/videos/my-playlist".to_string(),
        },
    );

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        *video_file_repository.deleted_calls.lock().unwrap(),
        vec![
            deleted("Season 2026/S2026E031501 - My Video.mp4"),
            deleted("Season 2026/S2026E031501 - My Video.nfo"),
            deleted("Season 2026/S2026E031501 - My Video.en.srt"),
        ]
    );
}

#[test]
fn it_should_delete_a_legacy_per_video_folder() {
    let db = TestDatabase::new();
    let video_file_repository = Arc::new(FakeVideoFileRepository::default());
    let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
        video_file_repository.clone(),
        event_publisher(&db),
        "/videos",
    ));

    let result = run(
        &task,
        &task_for(Some("My Video/My Video.mp4"), Some("My Video/My Video.jpg")),
    );

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        *video_file_repository.deleted_calls.lock().unwrap(),
        vec![deleted("My Video"), deleted("My Video")]
    );
}

#[test]
fn it_should_publish_video_file_deleted_after_deleting_a_video_file() {
    let db = TestDatabase::new();
    let task = DeleteVideoFileTask::new(VideoFileDeleter::new(
        Arc::new(FakeVideoFileRepository::new(Ok(true))),
        event_publisher(&db),
        "/videos",
    ));

    let result = run(&task, &task_for(Some("My Video.mp4"), None));

    assert_eq!(result, Ok(TaskOutcome::Done));
    let event = DomainEvent::VideoFileDeleted(VideoFileDeleted {
        output_dir: "/videos/my-playlist".to_string(),
    });
    assert_eq!(
        SqliteEventRepository::new(db.database(), Arc::new(FixedClock(epoch())))
            .list_eligible()
            .unwrap(),
        vec![ScheduledEvent {
            id: 1,
            event_type: event.event_type().to_string(),
            payload: event.payload().to_string(),
            retries: 0,
            run_at: epoch(),
            created_at: epoch(),
            updated_at: epoch(),
            last_error: None,
        }]
    );
}

fn task_for(filename: Option<&str>, thumbnail_filename: Option<&str>) -> Task {
    Task::DeleteVideoFile {
        filename: filename.map(str::to_string),
        thumbnail_filename: thumbnail_filename.map(str::to_string),
        subtitle_filename: None,
        output_dir: "/videos/my-playlist".to_string(),
    }
}

/// The `(output_dir, entry)` pair the fake records for one delete call.
fn deleted(entry: &str) -> (PathBuf, String) {
    (PathBuf::from("/videos/my-playlist"), entry.to_string())
}

fn run(task: &DeleteVideoFileTask, queued: &Task) -> Result<TaskOutcome, String> {
    task.handle(queued).map_err(|e| e.to_string())
}

fn event_publisher(db: &TestDatabase) -> Arc<dyn EventPublisher> {
    Arc::new(SqliteEventPublisher::new(
        db.database(),
        Arc::new(FixedClock(epoch())),
    ))
}

fn epoch() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(0, 0).unwrap()
}
