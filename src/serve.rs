use crate::cli::ytdlp_update;
use crate::domain::playlist::PlaylistService;
use crate::http::{self, AppState};
use crate::infrastructure::repositories::domain_events_consumer::DomainEventsConsumer;
use crate::infrastructure::repositories::sqlite_event_repository::{
    EventPublisher, EventRepository, SqliteEventRepository,
};
use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::{
    SqliteTaskRepository, TaskRepository,
};
use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
use crate::infrastructure::repositories::system_clock::SystemClock;
use crate::infrastructure::repositories::task_executor::TaskExecutor;
use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubeApiPlaylistItemsRepository;
use crate::infrastructure::repositories::youtube_playlist_repository::YoutubeApiPlaylistRepository;
use crate::{subscribers, tasks};
use anyhow::{Context, Result};
use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_DB_PATH: &str = "yarrtube.sqlite3";
const DEFAULT_SYNC_INTERVAL_SECONDS: i64 = 3600;
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

fn sync_interval_seconds() -> i64 {
    std::env::var("YARRTUBE_SYNC_INTERVAL_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_SYNC_INTERVAL_SECONDS)
}

fn run_startup_ytdlp_update() {
    match ytdlp_update::update(&ytdlp_update::target_path()) {
        Ok(()) => println!("[startup] yt-dlp self-update succeeded"),
        Err(e) => eprintln!("[startup] yt-dlp self-update failed: {e}"),
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
        Ok(()) => println!("[startup] database check succeeded"),
        Err(e) => eprintln!("[startup] database check failed: {e}"),
    }
}

fn youtube_api_key() -> String {
    std::env::var("YOUTUBE_API_KEY").unwrap_or_default()
}

fn run_startup_youtube_api_key_check() {
    if youtube_api_key().is_empty() {
        eprintln!(
            "[startup] warning: YOUTUBE_API_KEY is not set; POST /playlists will fail its YouTube lookup"
        );
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
    // Playlists and events share one connection so `insert_with_event`/
    // `delete_with_event` can wrap both writes in a single transaction.
    let playlist_events_conn = Arc::new(Mutex::new(open_connection()?));
    let playlist_repository = SqlitePlaylistRepository::new(playlist_events_conn.clone())
        .context("failed to initialize playlist repository")?;
    let event_repository = Arc::new(
        SqliteEventRepository::new(playlist_events_conn, Arc::new(SystemClock))
            .context("failed to initialize event repository")?,
    );

    let task_repository = Arc::new(
        SqliteTaskRepository::new(
            Arc::new(Mutex::new(open_connection()?)),
            Arc::new(SystemClock),
        )
        .context("failed to initialize task repository")?,
    );
    task_repository
        .recover_running()
        .context("failed to recover tasks left running from a previous run")?;

    let video_repository = SqliteVideoRepository::new(open_connection()?)
        .context("failed to initialize video repository")?;

    let playlist_service = PlaylistService::new(
        Arc::new(playlist_repository),
        Arc::new(YoutubeApiPlaylistRepository::new(youtube_api_key())),
        Arc::new(SystemClock),
        event_repository.clone() as Arc<dyn EventPublisher>,
        Arc::new(video_repository),
        Arc::new(YoutubeApiPlaylistItemsRepository::new(youtube_api_key())),
        task_repository.clone() as Arc<dyn TaskRepository>,
        sync_interval_seconds(),
    );

    let event_consumer = Arc::new(DomainEventsConsumer::new(
        event_repository as Arc<dyn EventRepository>,
        subscribers::registry(playlist_service.clone()),
    ));
    let task_executor = Arc::new(TaskExecutor::new(
        task_repository as Arc<dyn TaskRepository>,
        tasks::registry(playlist_service.clone()),
    ));

    Ok(Application {
        state: AppState { playlist_service },
        event_consumer,
        task_executor,
    })
}

async fn status() -> StatusCode {
    StatusCode::OK
}

async fn heartbeat_loop() {
    let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    loop {
        interval.tick().await;
        println!("[heartbeat] yarrtube daemon is alive");
    }
}

async fn serve_http(port: u16, state: AppState) -> Result<()> {
    let app = Router::new()
        .route("/status", get(status))
        .merge(http::playlists_router(state));
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .with_context(|| format!("failed to bind HTTP server on port {port}"))?;
    println!("[startup] HTTP server listening on 0.0.0.0:{port}");
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
        eprintln!("Error: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

pub fn run() -> ExitCode {
    // These use a blocking HTTP client, so they run before the tokio runtime
    // starts rather than inside it (a blocking client can't run on a tokio
    // worker thread).
    run_startup_ytdlp_update();
    run_startup_database_check();
    run_startup_youtube_api_key_check();

    let app = match build_application() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Error: failed to initialize application state: {e}");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Error: failed to start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    runtime.block_on(run_async(app))
}
