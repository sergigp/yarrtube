use crate::cli::ytdlp_update;
use crate::domain::playlist::PlaylistService;
use crate::domain::task::TaskService;
use crate::domain::video::VideoService;
use crate::http::{self, AppState};
use crate::infrastructure::repositories::domain_events_consumer::DomainEventsConsumer;
use crate::infrastructure::repositories::filesystem_video_file_repository::FilesystemVideoFileRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::{
    PlaylistRepository, SqlitePlaylistRepository,
};
use crate::infrastructure::repositories::sqlite_task_repository::{
    SqliteTaskRepository, TaskRepository,
};
use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
use crate::infrastructure::repositories::task_executor::TaskExecutor;
use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubeApiPlaylistItemsRepository;
use crate::infrastructure::repositories::youtube_playlist_repository::YoutubeApiPlaylistRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::YtDlpVideoDownloaderRepository;
use crate::infrastructure::repositories::youtube_video_repository::YoutubeApiVideoRepository;
use crate::infrastructure::shared::domain_events::event_publisher::{
    EventPublisher, SqliteEventPublisher,
};
use crate::infrastructure::shared::domain_events::event_repository::{
    EventRepository, SqliteEventRepository,
};
use crate::infrastructure::shared::system_clock::SystemClock;
use crate::infrastructure::shared::web_assets::WebAssets;
use crate::{subscribers, tasks};
use anyhow::{Context, Result};
use axum::Router;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
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
const DEFAULT_VIDEOS_PATH: &str = "/videos";
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

fn videos_path() -> String {
    std::env::var("YARRTUBE_VIDEOS_PATH").unwrap_or_else(|_| DEFAULT_VIDEOS_PATH.to_string())
}

fn run_startup_ytdlp_update() {
    match ytdlp_update::update(&ytdlp_update::target_path()) {
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

fn open_connection() -> Result<rusqlite::Connection> {
    rusqlite::Connection::open(db_path())
        .with_context(|| format!("failed to open database at {:?}", db_path()))
}

struct Application {
    state: AppState,
    event_consumer: Arc<DomainEventsConsumer>,
    task_executor: Arc<TaskExecutor>,
}

fn build_application() -> Result<Application> {
    let playlist_repository: Arc<dyn PlaylistRepository> = Arc::new(
        SqlitePlaylistRepository::new(open_connection()?)
            .context("failed to initialize playlist repository")?,
    );
    let event_publisher = Arc::new(
        SqliteEventPublisher::new(
            Arc::new(Mutex::new(open_connection()?)),
            Arc::new(SystemClock),
        )
        .context("failed to initialize event publisher")?,
    );
    let event_repository = Arc::new(
        SqliteEventRepository::new(Arc::new(Mutex::new(open_connection()?)))
            .context("failed to initialize event repository")?,
    );

    let task_repository = Arc::new(
        SqliteTaskRepository::new(
            Arc::new(Mutex::new(open_connection()?)),
            Arc::new(SystemClock),
        )
        .context("failed to initialize task repository")?,
    );

    let video_repository = SqliteVideoRepository::new(open_connection()?)
        .context("failed to initialize video repository")?;

    let task_service = TaskService::new(task_repository.clone() as Arc<dyn TaskRepository>);

    let playlist_service = PlaylistService::new(
        playlist_repository.clone(),
        Arc::new(YoutubeApiPlaylistRepository::new(youtube_api_key())),
        event_publisher.clone() as Arc<dyn EventPublisher>,
        Arc::new(SystemClock),
    );
    let video_service = VideoService::new(
        playlist_repository.clone(),
        Arc::new(video_repository),
        Arc::new(YoutubeApiPlaylistItemsRepository::new(youtube_api_key())),
        Arc::new(YoutubeApiVideoRepository::new(youtube_api_key())),
        event_publisher as Arc<dyn EventPublisher>,
        task_repository.clone() as Arc<dyn TaskRepository>,
        Arc::new(YtDlpVideoDownloaderRepository),
        Arc::new(FilesystemVideoFileRepository),
        Arc::new(SystemClock),
        reconcile_interval_seconds(),
        videos_path(),
    );

    let event_consumer = Arc::new(DomainEventsConsumer::new(
        event_repository as Arc<dyn EventRepository>,
        subscribers::registry(
            video_service.clone(),
            playlist_repository,
            task_repository.clone() as Arc<dyn TaskRepository>,
            Arc::new(SystemClock),
        ),
        Arc::new(SystemClock),
    ));
    let task_executor = Arc::new(TaskExecutor::new(
        task_repository as Arc<dyn TaskRepository>,
        tasks::registry(video_service.clone()),
        Arc::new(SystemClock),
    ));
    task_executor
        .recover_stuck_tasks()
        .context("failed to recover tasks left running from a previous run")?;

    Ok(Application {
        state: AppState {
            playlist_service,
            video_service,
            task_service,
        },
        event_consumer,
        task_executor,
    })
}

async fn status() -> StatusCode {
    StatusCode::OK
}

/// Serves the embedded single-page application: `index.html` for `/`, the
/// matching embedded file for any other path, `404` for anything unmatched.
/// Mounted as the router's fallback, after `/api` and `/status`.
async fn serve_spa(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match WebAssets::get(path) {
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

    fn unique_temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "yarrtube-serve-test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
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
    async fn it_should_return_404_for_an_unknown_path() {
        let response = get(spa_router(), "/does-not-exist.js").await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
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
}
