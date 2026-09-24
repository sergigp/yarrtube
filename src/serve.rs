use crate::application::http::{self, ApiServices, VideosRoot};
use crate::application::{subscribers, tasks};
use crate::domain::channel::ChannelService;
use crate::domain::services::{
    ChannelVideoReconciler, DirectorySearcher, PlaylistCreator, PlaylistDeleter, PlaylistSearcher,
    PlaylistVideoReconciler, TaskViewSearcher, ThumbnailFetcher, VideoDownloader, VideoFileDeleter,
    VideoSearcher,
};
use crate::domain::task::Task;
use crate::infrastructure::client::ytdlp_updater::{RealYtdlpUpdater, YtdlpUpdater, target_path};
use crate::infrastructure::infrastructure_container::{
    InfrastructureContainer, InfrastructureSettings,
};
use crate::infrastructure::repositories::domain_events_consumer::DomainEventsConsumer;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::task_executor::TaskExecutor;
use crate::infrastructure::shared::web_assets::WebAssets;
use crate::infrastructure::shared::{sqlite_connection, sqlite_migrations};
use anyhow::{Context, Result};
use axum::Router;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
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
/// The parent directories the add dialog's folder browser defaults to, seeded
/// under the videos root at startup so the browser is never empty on a fresh
/// install.
const DEFAULT_STORAGE_DIRECTORIES: &[&str] = &["playlists", "channels"];
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

/// Ensures the default parent storage directories exist under the videos
/// root. A directory that already exists is left untouched, contents and all,
/// since `create_dir_all` succeeds on one that is already there.
///
/// A failure is logged and skipped rather than fatal, consistent with the
/// other startup checks, so a read-only or not-yet-ready mount does not take
/// the service down.
fn create_default_storage_directories(path: &std::path::Path) {
    DEFAULT_STORAGE_DIRECTORIES
        .iter()
        .map(|name| path.join(name))
        .map(|dir| (std::fs::create_dir_all(&dir), dir))
        .for_each(|(result, dir)| match result {
            Ok(()) => info!(directory = ?dir, "default storage directory is present"),
            Err(e) => {
                error!(directory = ?dir, error = %e, "failed to create default storage directory")
            }
        });
}

/// Runs at daemon startup rather than at image build time: the videos root is
/// a mount point, and a host directory mounted there at container start
/// replaces the path entirely, masking anything the image created under it.
fn run_startup_storage_directories_check() {
    create_default_storage_directories(std::path::Path::new(&videos_path()));
}

fn youtube_api_key() -> String {
    std::env::var("YOUTUBE_API_KEY").unwrap_or_default()
}

fn run_startup_youtube_api_key_check() {
    if youtube_api_key().is_empty() {
        warn!("YOUTUBE_API_KEY is not set; POST /playlists will fail its YouTube lookup");
    }
}

/// Applies pending schema migrations once at startup, before any repository
/// opens its own connection (see `build_infrastructure`).
fn run_startup_migrations() -> Result<()> {
    let mut conn =
        sqlite_connection::open(&db_path()).context("failed to open database for migrations")?;
    sqlite_migrations::apply(&mut conn)
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

fn build_infrastructure() -> Result<InfrastructureContainer> {
    InfrastructureContainer::new(InfrastructureSettings {
        db_path: db_path(),
        youtube_api_key: youtube_api_key(),
        videos_path: PathBuf::from(videos_path()),
        avatars_path: avatars_path(),
        ytdlp_path: target_path(),
    })
}

/// Must run before the task executor starts polling: requeues tasks a
/// previous run left `running` and seeds the recurring yt-dlp self-update.
fn prepare_task_queue(
    infrastructure: &InfrastructureContainer,
    task_executor: &TaskExecutor,
) -> Result<()> {
    task_executor
        .recover_stuck_tasks()
        .context("failed to recover tasks left running from a previous run")?;

    let update_ytdlp_first_run_at = infrastructure.clock.now()
        + chrono::Duration::seconds(tasks::update_ytdlp_task::UPDATE_INTERVAL_SECONDS);
    schedule_update_ytdlp_if_absent(
        infrastructure.task_repository.as_ref(),
        update_ytdlp_first_run_at,
    )
    .context("failed to schedule the recurring yt-dlp self-update task")
}

fn api_services(infrastructure: &InfrastructureContainer) -> ApiServices {
    ApiServices {
        playlist_creator: PlaylistCreator::new(
            infrastructure.playlist_repository.clone(),
            infrastructure.youtube_playlist_repository.clone(),
            infrastructure.event_publisher.clone(),
            infrastructure.clock.clone(),
        ),
        playlist_deleter: PlaylistDeleter::new(
            infrastructure.playlist_repository.clone(),
            infrastructure.video_repository.clone(),
            infrastructure.playlist_video_repository.clone(),
            infrastructure.event_publisher.clone(),
        ),
        playlist_searcher: PlaylistSearcher::new(infrastructure.playlist_repository.clone()),
        playlist_video_reconciler: playlist_video_reconciler(infrastructure),
        video_searcher: VideoSearcher::new(
            infrastructure.playlist_repository.clone(),
            infrastructure.playlist_video_repository.clone(),
            infrastructure.channel_repository.clone(),
            infrastructure.channel_video_repository.clone(),
            infrastructure.video_repository.clone(),
        ),
        task_view_searcher: TaskViewSearcher::new(
            infrastructure.task_repository.clone(),
            infrastructure.playlist_repository.clone(),
            infrastructure.channel_repository.clone(),
            infrastructure.video_repository.clone(),
            infrastructure.playlist_video_repository.clone(),
            infrastructure.channel_video_repository.clone(),
        ),
        channel_service: ChannelService::new(
            infrastructure.channel_repository.clone(),
            infrastructure.youtube_channel_repository.clone(),
            infrastructure.channel_avatar_repository.clone(),
            infrastructure.video_repository.clone(),
            infrastructure.channel_video_repository.clone(),
            infrastructure.event_publisher.clone(),
            infrastructure.clock.clone(),
        ),
        channel_video_reconciler: channel_video_reconciler(infrastructure),
        directory_searcher: DirectorySearcher::new(infrastructure.directory_repository.clone()),
        videos_root: VideosRoot(videos_path()),
    }
}

fn event_consumer(infrastructure: &InfrastructureContainer) -> DomainEventsConsumer {
    DomainEventsConsumer::new(
        infrastructure.event_repository.clone(),
        subscribers::registry(
            playlist_video_reconciler(infrastructure),
            channel_video_reconciler(infrastructure),
            infrastructure.playlist_repository.clone(),
            infrastructure.channel_repository.clone(),
            infrastructure.task_repository.clone(),
            infrastructure.clock.clone(),
            videos_path(),
        ),
        infrastructure.clock.clone(),
    )
}

fn task_executor(infrastructure: &InfrastructureContainer) -> TaskExecutor {
    TaskExecutor::new(
        infrastructure.task_repository.clone(),
        tasks::registry(
            playlist_video_reconciler(infrastructure),
            channel_video_reconciler(infrastructure),
            video_downloader(infrastructure),
            VideoFileDeleter::new(infrastructure.video_file_repository.clone(), videos_path()),
            infrastructure.task_repository.clone(),
            infrastructure.clock.clone(),
            infrastructure.ytdlp_updater.clone(),
            target_path(),
        ),
        infrastructure.clock.clone(),
        retry_base_delay_seconds(),
    )
}

fn playlist_video_reconciler(infrastructure: &InfrastructureContainer) -> PlaylistVideoReconciler {
    PlaylistVideoReconciler::new(
        infrastructure.playlist_repository.clone(),
        infrastructure.video_repository.clone(),
        infrastructure.playlist_video_repository.clone(),
        infrastructure.youtube_playlist_items_repository.clone(),
        infrastructure.youtube_metadata_repository.clone(),
        infrastructure.video_metadata_repository.clone(),
        infrastructure.event_publisher.clone(),
        infrastructure.task_repository.clone(),
        infrastructure.video_file_repository.clone(),
        thumbnail_fetcher(infrastructure),
        infrastructure.clock.clone(),
        reconcile_interval_seconds(),
        videos_path(),
    )
}

fn channel_video_reconciler(infrastructure: &InfrastructureContainer) -> ChannelVideoReconciler {
    ChannelVideoReconciler::new(
        infrastructure.channel_repository.clone(),
        infrastructure.video_repository.clone(),
        infrastructure.channel_video_repository.clone(),
        infrastructure.channel_videos_repository.clone(),
        infrastructure.youtube_metadata_repository.clone(),
        infrastructure.video_metadata_repository.clone(),
        infrastructure.event_publisher.clone(),
        infrastructure.task_repository.clone(),
        infrastructure.video_file_repository.clone(),
        thumbnail_fetcher(infrastructure),
        infrastructure.clock.clone(),
        reconcile_interval_seconds(),
        videos_path(),
    )
}

fn video_downloader(infrastructure: &InfrastructureContainer) -> VideoDownloader {
    VideoDownloader::new(
        infrastructure.video_repository.clone(),
        infrastructure.video_downloader_repository.clone(),
        infrastructure.video_file_repository.clone(),
        infrastructure.playlist_video_repository.clone(),
        infrastructure.youtube_metadata_repository.clone(),
        infrastructure.video_metadata_repository.clone(),
        infrastructure.clock.clone(),
    )
}

fn thumbnail_fetcher(infrastructure: &InfrastructureContainer) -> Arc<ThumbnailFetcher> {
    Arc::new(ThumbnailFetcher::new(
        infrastructure.video_repository.clone(),
        infrastructure.video_downloader_repository.clone(),
        infrastructure.clock.clone(),
    ))
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

async fn serve_http(port: u16, api_services: ApiServices) -> Result<()> {
    let router = Router::new()
        .route("/status", get(status))
        .nest("/api", http::api_router(api_services))
        .nest_service("/media", ServeDir::new(videos_path()))
        .nest_service("/avatars", ServeDir::new(avatars_path()))
        .fallback(serve_spa);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .with_context(|| format!("failed to bind HTTP server on port {port}"))?;
    info!(port, "HTTP server listening on 0.0.0.0");
    axum::serve(listener, router)
        .await
        .context("HTTP server failed")?;
    Ok(())
}

async fn run_async(
    infrastructure: InfrastructureContainer,
    task_executor: Arc<TaskExecutor>,
) -> ExitCode {
    tokio::spawn(heartbeat_loop());
    tokio::spawn(Arc::new(event_consumer(&infrastructure)).run(BACKGROUND_POLL_INTERVAL));
    tokio::spawn(task_executor.run(BACKGROUND_POLL_INTERVAL));

    if let Err(e) = serve_http(port(), api_services(&infrastructure)).await {
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
    run_startup_storage_directories_check();

    if let Err(e) = run_startup_migrations() {
        error!(error = %e, "failed to apply database migrations");
        return ExitCode::FAILURE;
    }

    let infrastructure = match build_infrastructure() {
        Ok(infrastructure) => infrastructure,
        Err(e) => {
            error!(error = %e, "failed to initialize infrastructure");
            return ExitCode::FAILURE;
        }
    };
    let task_executor = Arc::new(task_executor(&infrastructure));
    if let Err(e) = prepare_task_queue(&infrastructure, &task_executor) {
        error!(error = %e, "failed to prepare the task queue");
        return ExitCode::FAILURE;
    }

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

    runtime.block_on(run_async(infrastructure, task_executor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use std::sync::Mutex;
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

    #[test]
    fn it_should_create_the_default_storage_directories_on_an_empty_videos_root() {
        let videos_root = unique_temp_dir("storage-directories-empty");
        std::fs::create_dir_all(&videos_root).unwrap();

        create_default_storage_directories(&videos_root);

        assert!(videos_root.join("playlists").is_dir());
        assert!(videos_root.join("channels").is_dir());
        std::fs::remove_dir_all(&videos_root).unwrap();
    }

    #[test]
    fn it_should_leave_existing_default_storage_directories_and_their_contents_untouched() {
        let videos_root = unique_temp_dir("storage-directories-existing");
        std::fs::create_dir_all(videos_root.join("playlists/music")).unwrap();
        std::fs::write(videos_root.join("playlists/music/My Video.mp4"), b"bytes").unwrap();

        create_default_storage_directories(&videos_root);

        assert!(videos_root.join("playlists/music").is_dir());
        assert_eq!(
            std::fs::read(videos_root.join("playlists/music/My Video.mp4")).unwrap(),
            b"bytes"
        );
        assert!(videos_root.join("channels").is_dir());
        std::fs::remove_dir_all(&videos_root).unwrap();
    }

    #[test]
    fn it_should_continue_starting_up_on_a_directory_creation_failure() {
        // A regular file where the videos root should be: every
        // `create_dir_all` under it fails, the way a read-only mount would.
        let videos_root = unique_temp_dir("storage-directories-failure").join("not-a-directory");
        std::fs::create_dir_all(videos_root.parent().unwrap()).unwrap();
        std::fs::write(&videos_root, b"").unwrap();

        create_default_storage_directories(&videos_root);

        assert!(!videos_root.join("playlists").exists());
        assert!(!videos_root.join("channels").exists());
        std::fs::remove_dir_all(videos_root.parent().unwrap()).unwrap();
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
