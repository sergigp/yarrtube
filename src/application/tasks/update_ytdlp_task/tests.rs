use super::*;
use crate::domain::task::{ScheduledTask, TaskStatus};
use crate::infrastructure::client::ytdlp_updater::FakeYtdlpUpdater;
use crate::infrastructure::repositories::sqlite_task_repository::SqliteTaskRepository;
use crate::infrastructure::shared::sqlite_connection::TestDatabase;
use crate::infrastructure::shared::system_clock::FixedClock;

#[test]
fn it_should_reschedule_after_success() {
    let db = TestDatabase::new();
    let task_repository = Arc::new(SqliteTaskRepository::new(
        db.database(),
        Arc::new(FixedClock(fixed_timestamp())),
    ));
    let task = UpdateYtdlpTask::new(
        Arc::new(FakeYtdlpUpdater::succeeding()),
        PathBuf::from("/app/bin/yt-dlp"),
        true,
        task_repository.clone(),
        Arc::new(FixedClock(fixed_timestamp())),
    );

    let result = run(&task, &Task::UpdateYtdlp);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_task(
            1,
            &Task::UpdateYtdlp,
            fixed_timestamp() + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS),
        )]
    );
}

#[test]
fn it_should_skip_the_update_and_reschedule_if_self_update_is_off() {
    let db = TestDatabase::new();
    let task_repository = Arc::new(SqliteTaskRepository::new(
        db.database(),
        Arc::new(FixedClock(fixed_timestamp())),
    ));
    let updater = Arc::new(FakeYtdlpUpdater::succeeding());
    let task = UpdateYtdlpTask::new(
        updater.clone(),
        PathBuf::from("/fake-youtube/yt-dlp"),
        false,
        task_repository.clone(),
        Arc::new(FixedClock(fixed_timestamp())),
    );

    let result = run(&task, &Task::UpdateYtdlp);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(*updater.updated.lock().unwrap(), Vec::<PathBuf>::new());
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_task(
            1,
            &Task::UpdateYtdlp,
            fixed_timestamp() + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS),
        )]
    );
}

#[test]
fn it_should_reschedule_after_failure() {
    let db = TestDatabase::new();
    let task_repository = Arc::new(SqliteTaskRepository::new(
        db.database(),
        Arc::new(FixedClock(fixed_timestamp())),
    ));
    let task = UpdateYtdlpTask::new(
        Arc::new(FakeYtdlpUpdater::failing()),
        PathBuf::from("/app/bin/yt-dlp"),
        true,
        task_repository.clone(),
        Arc::new(FixedClock(fixed_timestamp())),
    );

    let result = run(&task, &Task::UpdateYtdlp);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_task(
            1,
            &Task::UpdateYtdlp,
            fixed_timestamp() + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS),
        )]
    );
}

#[test]
fn it_should_seed_the_task_when_none_is_scheduled() {
    let db = TestDatabase::new();
    let task_repository =
        SqliteTaskRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));

    let result = schedule_update_ytdlp_if_absent(&task_repository, seed_run_at());

    assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_task(1, &Task::UpdateYtdlp, seed_run_at())]
    );
}

#[test]
fn it_should_not_seed_if_one_is_pending() {
    let db = TestDatabase::new();
    let task_repository =
        SqliteTaskRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
    task_repository
        .schedule(&Task::UpdateYtdlp, fixed_timestamp())
        .unwrap();

    let result = schedule_update_ytdlp_if_absent(&task_repository, seed_run_at());

    assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_task(1, &Task::UpdateYtdlp, fixed_timestamp())]
    );
}

#[test]
fn it_should_not_seed_if_one_is_running() {
    let db = TestDatabase::new();
    let task_repository =
        SqliteTaskRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
    task_repository
        .schedule(&Task::UpdateYtdlp, fixed_timestamp())
        .unwrap();
    task_repository
        .update(&pending_task(1, &Task::UpdateYtdlp, fixed_timestamp()).start(fixed_timestamp()))
        .unwrap();

    let result = schedule_update_ytdlp_if_absent(&task_repository, seed_run_at());

    assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![ScheduledTask {
            status: TaskStatus::Running,
            ..pending_task(1, &Task::UpdateYtdlp, fixed_timestamp())
        }]
    );
}

#[test]
fn it_should_seed_if_only_other_task_types_are_scheduled() {
    let db = TestDatabase::new();
    let task_repository =
        SqliteTaskRepository::new(db.database(), Arc::new(FixedClock(fixed_timestamp())));
    let reconcile_playlist = Task::ReconcilePlaylist {
        playlist_id: "PL1".to_string(),
    };
    task_repository
        .schedule(&reconcile_playlist, fixed_timestamp())
        .unwrap();

    let result = schedule_update_ytdlp_if_absent(&task_repository, seed_run_at());

    assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![
            pending_task(1, &reconcile_playlist, fixed_timestamp()),
            pending_task(2, &Task::UpdateYtdlp, seed_run_at()),
        ]
    );
}

fn pending_task(id: i64, task: &Task, run_at: DateTime<Utc>) -> ScheduledTask {
    ScheduledTask {
        id,
        task: task.clone(),
        status: TaskStatus::Pending,
        retries: 0,
        run_at,
        created_at: fixed_timestamp(),
        updated_at: fixed_timestamp(),
        last_error: None,
    }
}

fn fixed_timestamp() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
}

fn seed_run_at() -> DateTime<Utc> {
    fixed_timestamp() + chrono::Duration::seconds(UPDATE_INTERVAL_SECONDS)
}

fn run(task: &UpdateYtdlpTask, queued: &Task) -> Result<TaskOutcome, String> {
    task.handle(queued).map_err(|e| e.to_string())
}
