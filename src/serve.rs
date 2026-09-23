use crate::application::http::{self, AppState};
use crate::application::{subscribers, tasks};
use crate::domain::channel::ChannelService;
use crate::domain::services::{
    ChannelVideoReconciler, PlaylistCreator, PlaylistDeleter, PlaylistSearcher, TaskViewSearcher,
    ThumbnailFetcher, VideoDownloader, VideoFileDeleter, VideoReconciler, VideoSearcher,
};
use crate::domain::task::Task;
use crate::infrastructure::client::ytdlp_updater::{RealYtdlpUpdater, YtdlpUpdater, target_path};
use crate::infrastructure::repositories::domain_events_consumer::DomainEventsConsumer;
use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FilesystemChannelAvatarRepository;
use crate::infrastructure::repositories::filesystem_video_file_repository::FilesystemVideoFileRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::{
    ChannelRepository, SqliteChannelRepository,
};
use crate::infrastructure::repositories::sqlite_channel_video_repository::SqliteChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::{
    PlaylistRepository, SqlitePlaylistRepository,
};
use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::{
    SqliteTaskRepository, TaskRepository,
};
use crate::infrastructure::repositories::sqlite_video_metadata_repository::SqliteVideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
use crate::infrastructure::repositories::task_executor::TaskExecutor;
use crate::infrastructure::repositories::youtube_channel_repository::YoutubeApiChannelRepository;
use crate::infrastructure::repositories::youtube_channel_videos_repository::YtDlpChannelVideosRepository;
use crate::infrastructure::repositories::youtube_metadata_repository::YoutubeApiMetadataRepository;
use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubeApiPlaylistItemsRepository;
use crate::infrastructure::repositories::youtube_playlist_repository::YoutubeApiPlaylistRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::YtDlpVideoDownloaderRepository;
use crate::infrastructure::shared::domain_events::event_publisher::{
    EventPublisher, SqliteEventPublisher,
};
use crate::infrastructure::shared::domain_events::event_repository::{
    EventRepository, SqliteEventRepository,
};
use crate::infrastructure::shared::sqlite_migrations;
use crate::infrastructure::shared::system_clock::{Clock, SystemClock};
use crate::infrastructure::shared::web_assets::WebAssets;
use anyhow::{Context, Result};
use axum::Router;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower_http::services::ServeDir;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_DB_PATH: &str = "yarrtube.sqlite3";
const DEFAULT_RECONCILE_INTERVAL_SECONDS: i64 = 3600;
const DEFAULT_RETRY_BASE_DELAY_SECONDS: i64 = 150;
const DEFAULT_VIDEOS_PATH: &str = "/videos";
const DEFAULT_AVATARS_PATH: &str = "avatars";
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);
const BACKGROUND_POLL_INTERVAL: Duration = Duration::from_secs(5);

fn port() -> u16 {
    std::env::var("YARRTUBE_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

fn db_path() -> PathBuf {
    std::env::var("YARRTUBE_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_DB_PATH))
}

fn reconcile_interval_seconds() -> i64 {
    std::env::var("YARRTUBE_RECONCILE_INTERVAL_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RECONCILE_INTERVAL_SECONDS)
}

fn retry_base_delay_seconds() -> i64 {
    std::env::var("YARRTUBE_RETRY_BASE_DELAY_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RETRY_BASE_DELAY_SECONDS)
}

fn videos_path() -> String {
    std::env::var("YARRTUBE_VIDEOS_PATH").unwrap_or_else(|_| DEFAULT_VIDEOS_PATH.to_string())
}

/// Container-local and un-mounted, unlike `videos_path()` — see design.md's
/// "Avatar bytes are downloaded and written to a new container-local
/// `avatars/` directory" decision: it lives alongside the (also
/// container-local, un-mounted) SQLite database, not under the mounted
/// videos root.
fn avatars_path() -> PathBuf {
    std::env::var("YARRTUBE_AVATARS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_AVATARS_PATH))
}

fn run_startup_ytdlp_update() {
    match RealYtdlpUpdater.update(&target_path()) {
        Ok(()) => info!("yt-dlp self-update succeeded"),
        Err(e) => error!(error = %e, "yt-dlp self-update failed"),
    }
}

fn check_database(path: &std::path::Path) -> Result<()> {
    let conn = rusqlite::Connection::open(path)
        .with_context(|| format!("failed to open database at {path:?}"))?;
    conn.query_row("SELECT 1", [], |_| Ok(()))
        .context("failed to run connectivity check query")?;
    Ok(())
}

fn run_startup_database_check() {
    match check_database(&db_path()) {
        Ok(()) => info!("database check succeeded"),
        Err(e) => error!(error = %e, "database check failed"),
    }
}

fn youtube_api_key() -> String {
    std::env::var("YOUTUBE_API_KEY").unwrap_or_default()
}

fn run_startup_youtube_api_key_check() {
    if youtube_api_key().is_empty() {
        warn!("YOUTUBE_API_KEY is not set; POST /playlists will fail its YouTube lookup");
    }
}

/// Opens a connection to the shared database file. Each repository gets its
/// own connection (see `build_application`), so WAL mode lets readers on one
/// connection proceed while another holds the write lock, and `busy_timeout`
/// makes SQLite retry internally for up to 5s instead of immediately
/// returning `SQLITE_BUSY` ("database is locked") when two connections
/// briefly contend for the write lock.
fn open_connection() -> Result<rusqlite::Connection> {
    let conn = rusqlite::Connection::open(db_path())
        .with_context(|| format!("failed to open database at {:?}", db_path()))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .context("failed to enable WAL journal mode")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .context("failed to set busy timeout")?;
    Ok(conn)
}

/// Applies pending schema migrations once at startup, before any repository
/// opens its own connection (see `build_application`).
fn run_startup_migrations() -> Result<()> {
    let mut conn = open_connection().context("failed to open database for migrations")?;
    sqlite_migrations::apply(&mut conn)
}

struct Application {
    state: AppState,
    event_consumer: Arc<DomainEventsConsumer>,
    task_executor: Arc<TaskExecutor>,
}

/// Seeds the recurring `update_ytdlp` task chain at `run_at`, unless a
/// non-terminal (`pending` or `running`) one is already scheduled — a
/// restarted daemon must not stack up additional hourly self-update chains
/// alongside one that already self-perpetuates forever.
fn schedule_update_ytdlp_if_absent(
    task_repository: &dyn TaskRepository,
    run_at: DateTime<Utc>,
) -> Result<()> {
    let existing = task_repository
        .list_non_completed()?
        .into_iter()
        .find(|task| task.task_type == Task::UpdateYtdlp.task_type());

    match existing {
        Some(task) => info!(
            task_id = task.id,
            "recurring yt-dlp self-update task already scheduled, skipping seed"
        ),
        None => {
            task_repository.schedule(&Task::UpdateYtdlp, run_at)?;
            info!(run_at = %run_at, "scheduled recurring yt-dlp self-update task");
        }
    }
    Ok(())
}

fn build_application() -> Result<Application> {
    let playlist_repository: Arc<dyn PlaylistRepository> =
        Arc::new(SqlitePlaylistRepository::new(open_connection()?));
    let event_publisher = Arc::new(SqliteEventPublisher::new(
        Arc::new(Mutex::new(open_connection()?)),
        Arc::new(SystemClock),
    ));
    let event_repository = Arc::new(SqliteEventRepository::new(Arc::new(Mutex::new(
        open_connection()?,
    ))));

    let task_repository = Arc::new(SqliteTaskRepository::new(
        Arc::new(Mutex::new(open_connection()?)),
        Arc::new(SystemClock),
    ));

    let video_repository = Arc::new(SqliteVideoRepository::new(open_connection()?));

    let channel_repository: Arc<dyn ChannelRepository> =
        Arc::new(SqliteChannelRepository::new(open_connection()?));

    let playlist_video_repository =
        Arc::new(SqlitePlaylistVideoRepository::new(open_connection()?));
    let channel_video_repository = Arc::new(SqliteChannelVideoRepository::new(open_connection()?));

    let task_view_searcher = TaskViewSearcher::new(
        task_repository.clone(),
        playlist_repository.clone(),
        channel_repository.clone(),
        video_repository.clone(),
        playlist_video_repository.clone(),
        channel_video_repository.clone(),
    );

    let playlist_creator = PlaylistCreator::new(
        playlist_repository.clone(),
        Arc::new(YoutubeApiPlaylistRepository::new(youtube_api_key())),
        event_publisher.clone() as Arc<dyn EventPublisher>,
        Arc::new(SystemClock),
    );
    let playlist_deleter = PlaylistDeleter::new(
        playlist_repository.clone(),
        video_repository.clone(),
        playlist_video_repository.clone(),
        event_publisher.clone() as Arc<dyn EventPublisher>,
    );
    let playlist_searcher = PlaylistSearcher::new(playlist_repository.clone());
    let channel_service = ChannelService::new(
        channel_repository.clone(),
        Arc::new(YoutubeApiChannelRepository::new(youtube_api_key())),
        Arc::new(FilesystemChannelAvatarRepository::new(avatars_path())),
        video_repository.clone(),
        channel_video_repository.clone(),
        event_publisher.clone() as Arc<dyn EventPublisher>,
        Arc::new(SystemClock),
    );
    let event_publisher = event_publisher as Arc<dyn EventPublisher>;
    let task_repository = task_repository as Arc<dyn TaskRepository>;
    let video_file_repository = Arc::new(FilesystemVideoFileRepository);
    let youtube_metadata_repository =
        Arc::new(YoutubeApiMetadataRepository::new(youtube_api_key()));
    let video_metadata_repository =
        Arc::new(SqliteVideoMetadataRepository::new(open_connection()?));

    let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
        video_repository.clone(),
        Arc::new(YtDlpVideoDownloaderRepository::new(target_path())),
        Arc::new(SystemClock),
    ));
    let video_reconciler = VideoReconciler::new(
        playlist_repository.clone(),
        video_repository.clone(),
        playlist_video_repository.clone(),
        Arc::new(YoutubeApiPlaylistItemsRepository::new(youtube_api_key())),
        youtube_metadata_repository.clone(),
        video_metadata_repository.clone(),
        event_publisher.clone(),
        task_repository.clone(),
        video_file_repository.clone(),
        thumbnail_fetcher.clone(),
        Arc::new(SystemClock),
        reconcile_interval_seconds(),
        videos_path(),
    );
    let channel_video_reconciler = ChannelVideoReconciler::new(
        channel_repository.clone(),
        video_repository.clone(),
        channel_video_repository.clone(),
        Arc::new(YtDlpChannelVideosRepository::new(target_path())),
        youtube_metadata_repository.clone(),
        video_metadata_repository.clone(),
        event_publisher.clone(),
        task_repository.clone(),
        video_file_repository.clone(),
        thumbnail_fetcher.clone(),
        Arc::new(SystemClock),
        reconcile_interval_seconds(),
        videos_path(),
    );
    let video_downloader = VideoDownloader::new(
        video_repository.clone(),
        Arc::new(YtDlpVideoDownloaderRepository::new(target_path())),
        video_file_repository.clone(),
        playlist_video_repository.clone(),
        youtube_metadata_repository.clone(),
        video_metadata_repository.clone(),
        Arc::new(SystemClock),
    );
    let video_file_deleter = VideoFileDeleter::new(video_file_repository, videos_path());
    let video_searcher = VideoSearcher::new(
        playlist_repository.clone(),
        playlist_video_repository,
        channel_repository.clone(),
        channel_video_repository.clone(),
        video_repository,
    );

    let event_consumer = Arc::new(DomainEventsConsumer::new(
        event_repository as Arc<dyn EventRepository>,
        subscribers::registry(
            video_reconciler.clone(),
            channel_video_reconciler.clone(),
            playlist_repository,
            channel_repository,
            task_repository.clone(),
            Arc::new(SystemClock),
            videos_path(),
        ),
        Arc::new(SystemClock),
    ));
    let task_executor = Arc::new(TaskExecutor::new(
        task_repository.clone(),
        tasks::registry(
            video_reconciler.clone(),
            channel_video_reconciler.clone(),
            video_downloader.clone(),
            video_file_deleter.clone(),
            task_repository.clone(),
            Arc::new(SystemClock),
            Arc::new(RealYtdlpUpdater),
            target_path(),
        ),
        Arc::new(SystemClock),
        retry_base_delay_seconds(),
    ));
    task_executor
        .recover_stuck_tasks()
        .context("failed to recover tasks left running from a previous run")?;

    let update_ytdlp_first_run_at = SystemClock.now()
        + chrono::Duration::seconds(tasks::update_ytdlp_task::UPDATE_INTERVAL_SECONDS);
    schedule_update_ytdlp_if_absent(task_repository.as_ref(), update_ytdlp_first_run_at)
        .context("failed to schedule the recurring yt-dlp self-update task")?;

    Ok(Application {
        state: AppState {
            playlist_creator,
            playlist_deleter,
            playlist_searcher,
            video_reconciler,
            video_searcher,
            task_view_searcher,
            channel_service,
            channel_video_reconciler,
        },
        event_consumer,
        task_executor,
    })
}

async fn status() -> StatusCode {
    StatusCode::OK
}

/// Serves the embedded single-page application: the matching embedded file
/// for a path that names one (e.g. a hashed JS/CSS asset), `index.html` for
/// `/` or any other unmatched path (so the client-side router can handle
/// routes like `/playlists/:id`), `404` only if `index.html` itself is
/// missing from the embedded bundle. Mounted as the router's fallback, after
/// `/api` and `/status`.
async fn serve_spa(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match WebAssets::get(path).or_else(|| WebAssets::get("index.html")) {
        Some(file) => {
            let mime = file.metadata.mimetype();
            ([(header::CONTENT_TYPE, mime)], file.data).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn heartbeat_loop() {
    let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    loop {
        interval.tick().await;
        info!("yarrtube daemon is alive");
    }
}

async fn serve_http(port: u16, state: AppState) -> Result<()> {
    let app = Router::new()
        .route("/status", get(status))
        .nest("/api", http::api_router(state))
        .nest_service("/media", ServeDir::new(videos_path()))
        .nest_service("/avatars", ServeDir::new(avatars_path()))
        .fallback(serve_spa);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .with_context(|| format!("failed to bind HTTP server on port {port}"))?;
    info!(port, "HTTP server listening on 0.0.0.0");
    axum::serve(listener, app)
        .await
        .context("HTTP server failed")?;
    Ok(())
}

async fn run_async(app: Application) -> ExitCode {
    tokio::spawn(heartbeat_loop());
    tokio::spawn(app.event_consumer.run(BACKGROUND_POLL_INTERVAL));
    tokio::spawn(app.task_executor.run(BACKGROUND_POLL_INTERVAL));

    if let Err(e) = serve_http(port(), app.state).await {
        error!(error = %e, "HTTP server failed");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

pub fn run() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    // These use a blocking HTTP client, so they run before the tokio runtime
    // starts rather than inside it (a blocking client can't run on a tokio
    // worker thread).
    run_startup_ytdlp_update();
    run_startup_database_check();
    run_startup_youtube_api_key_check();

    if let Err(e) = run_startup_migrations() {
        error!(error = %e, "failed to apply database migrations");
        return ExitCode::FAILURE;
    }

    let app = match build_application() {
        Ok(app) => app,
        Err(e) => {
            error!(error = %e, "failed to initialize application state");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            error!(error = %e, "failed to start async runtime");
            return ExitCode::FAILURE;
        }
    };

    runtime.block_on(run_async(app))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    fn spa_router() -> Router {
        Router::new().fallback(serve_spa)
    }

    fn media_router(root: &std::path::Path) -> Router {
        Router::new()
            .nest_service("/media", ServeDir::new(root))
            .fallback(serve_spa)
    }

    fn avatars_router(root: &std::path::Path) -> Router {
        Router::new()
            .nest_service("/avatars", ServeDir::new(root))
            .fallback(serve_spa)
    }

    /// Tests run in parallel threads within one process, so the PID is the
    /// same for all of them, and `SystemTime::now()`'s resolution isn't
    /// guaranteed to be finer than the gap between two threads calling this
    /// concurrently. A process-wide counter guarantees every call gets a
    /// distinct suffix regardless of clock resolution.
    static UNIQUE_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn unique_temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "yarrtube-serve-test-{label}-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            UNIQUE_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
    }

    async fn get(router: Router, path: &str) -> Response {
        router
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn it_should_serve_the_embedded_index_html_at_root() {
        let response = get(spa_router(), "/").await;

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body.contains("<div id=\"root\">"));
    }

    #[tokio::test]
    async fn it_should_serve_a_known_embedded_asset_by_path() {
        let asset_path = WebAssets::iter()
            .find(|p| p.as_ref() != "index.html")
            .expect("web/dist/ must contain at least one built asset");

        let response = get(spa_router(), &format!("/{asset_path}")).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn it_should_serve_index_html_for_an_unmatched_client_route() {
        let response = get(spa_router(), "/playlists/some-playlist-id").await;

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body.contains("<div id=\"root\">"));
    }

    #[tokio::test]
    async fn it_should_serve_a_known_file_under_the_mounted_media_root() {
        let root = unique_temp_dir("known-file");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("video.mp4"), b"fake video bytes").unwrap();

        let response = get(media_router(&root), "/media/video.mp4").await;

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(bytes.as_ref(), b"fake video bytes");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[tokio::test]
    async fn it_should_serve_a_channel_owned_videos_file_under_its_nested_storage_path() {
        // The `/media` mount is container-agnostic: a channel's storage path
        // (e.g. "creators/somechannel", see `Channel.path`) is served the
        // same way a playlist's is, since both are just subdirectories under
        // the configured videos root.
        let root = unique_temp_dir("channel-owned-file");
        let channel_dir = root.join("creators/somechannel");
        std::fs::create_dir_all(&channel_dir).unwrap();
        std::fs::write(channel_dir.join("video.mp4"), b"fake channel video bytes").unwrap();

        let response = get(media_router(&root), "/media/creators/somechannel/video.mp4").await;

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(bytes.as_ref(), b"fake channel video bytes");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[tokio::test]
    async fn it_should_serve_a_byte_range_of_a_known_file() {
        let root = unique_temp_dir("byte-range");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("video.mp4"), b"0123456789").unwrap();

        let response = media_router(&root)
            .oneshot(
                Request::builder()
                    .uri("/media/video.mp4")
                    .header(header::RANGE, "bytes=2-5")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(bytes.as_ref(), b"2345");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[tokio::test]
    async fn it_should_not_serve_a_file_outside_the_media_root_via_path_traversal() {
        let root = unique_temp_dir("traversal-root");
        std::fs::create_dir_all(&root).unwrap();
        let secret_name = format!("yarrtube-serve-test-secret-{}.txt", std::process::id());
        let secret_path = root.parent().unwrap().join(&secret_name);
        std::fs::write(&secret_path, b"outside the root").unwrap();

        let response = media_router(&root)
            .oneshot(
                Request::builder()
                    .uri(format!("/media/..%2f{secret_name}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_ne!(response.status(), StatusCode::OK);

        std::fs::remove_dir_all(&root).unwrap();
        std::fs::remove_file(&secret_path).unwrap();
    }

    #[tokio::test]
    async fn it_should_serve_a_known_file_under_the_mounted_avatars_root() {
        let root = unique_temp_dir("avatars-known-file");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("@somechannel.jpg"), b"fake avatar bytes").unwrap();

        let response = get(avatars_router(&root), "/avatars/@somechannel.jpg").await;

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(bytes.as_ref(), b"fake avatar bytes");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[tokio::test]
    async fn it_should_not_serve_a_file_outside_the_avatars_root_via_path_traversal() {
        let root = unique_temp_dir("avatars-traversal-root");
        std::fs::create_dir_all(&root).unwrap();
        let secret_name = format!(
            "yarrtube-serve-test-avatars-secret-{}.txt",
            std::process::id()
        );
        let secret_path = root.parent().unwrap().join(&secret_name);
        std::fs::write(&secret_path, b"outside the root").unwrap();

        let response = avatars_router(&root)
            .oneshot(
                Request::builder()
                    .uri(format!("/avatars/..%2f{secret_name}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_ne!(response.status(), StatusCode::OK);

        std::fs::remove_dir_all(&root).unwrap();
        std::fs::remove_file(&secret_path).unwrap();
    }

    #[tokio::test]
    async fn it_should_still_serve_the_spa_root_and_a_known_asset_with_media_mounted() {
        let root = unique_temp_dir("spa-alongside");
        std::fs::create_dir_all(&root).unwrap();

        let response = get(media_router(&root), "/").await;
        assert_eq!(response.status(), StatusCode::OK);

        let asset_path = WebAssets::iter()
            .find(|p| p.as_ref() != "index.html")
            .expect("web/dist/ must contain at least one built asset");
        let response = get(media_router(&root), &format!("/{asset_path}")).await;
        assert_eq!(response.status(), StatusCode::OK);

        std::fs::remove_dir_all(&root).unwrap();
    }

    use crate::domain::task::{ScheduledTask, TaskStatus};

    #[derive(Default)]
    struct FakeTaskRepository {
        non_completed: Vec<ScheduledTask>,
        scheduled: Mutex<Vec<Task>>,
    }

    fn non_completed_task(id: i64, task_type: &str, status: TaskStatus) -> ScheduledTask {
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        ScheduledTask {
            id,
            task_type: task_type.to_string(),
            payload: "{}".to_string(),
            status,
            retries: 0,
            run_at: now,
            created_at: now,
            updated_at: now,
            last_error: None,
        }
    }

    impl TaskRepository for FakeTaskRepository {
        fn schedule(&self, task: &Task, _run_at: DateTime<Utc>) -> anyhow::Result<()> {
            self.scheduled.lock().unwrap().push(task.clone());
            Ok(())
        }

        fn list_eligible(&self) -> anyhow::Result<Vec<ScheduledTask>> {
            unimplemented!("not exercised by the seeding guard")
        }

        fn list_running(&self) -> anyhow::Result<Vec<ScheduledTask>> {
            unimplemented!("not exercised by the seeding guard")
        }

        fn list_non_completed(&self) -> anyhow::Result<Vec<ScheduledTask>> {
            Ok(self.non_completed.clone())
        }

        fn update(&self, _task: &ScheduledTask) -> anyhow::Result<()> {
            unimplemented!("not exercised by the seeding guard")
        }

        fn delete(&self, _id: i64) -> anyhow::Result<()> {
            unimplemented!("not exercised by the seeding guard")
        }

        fn dead_letter(&self, _task: &crate::domain::task::DeadLetteredTask) -> anyhow::Result<()> {
            unimplemented!("not exercised by the seeding guard")
        }
    }

    fn run_at() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_003_600, 0).unwrap()
    }

    #[test]
    fn it_should_schedule_the_recurring_task_when_none_is_non_completed() {
        let repository = FakeTaskRepository::default();

        schedule_update_ytdlp_if_absent(&repository, run_at()).unwrap();

        let scheduled = repository.scheduled.lock().unwrap();
        assert_eq!(scheduled.as_slice(), [Task::UpdateYtdlp]);
    }

    #[test]
    fn it_should_skip_scheduling_when_a_pending_update_ytdlp_task_already_exists() {
        let repository = FakeTaskRepository {
            non_completed: vec![non_completed_task(1, "update_ytdlp", TaskStatus::Pending)],
            ..Default::default()
        };

        schedule_update_ytdlp_if_absent(&repository, run_at()).unwrap();

        assert!(repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_skip_scheduling_when_a_running_update_ytdlp_task_already_exists() {
        let repository = FakeTaskRepository {
            non_completed: vec![non_completed_task(1, "update_ytdlp", TaskStatus::Running)],
            ..Default::default()
        };

        schedule_update_ytdlp_if_absent(&repository, run_at()).unwrap();

        assert!(repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_schedule_when_only_other_task_types_are_non_completed() {
        let repository = FakeTaskRepository {
            non_completed: vec![non_completed_task(
                1,
                "reconcile_playlist",
                TaskStatus::Pending,
            )],
            ..Default::default()
        };

        schedule_update_ytdlp_if_absent(&repository, run_at()).unwrap();

        let scheduled = repository.scheduled.lock().unwrap();
        assert_eq!(scheduled.as_slice(), [Task::UpdateYtdlp]);
    }
}
