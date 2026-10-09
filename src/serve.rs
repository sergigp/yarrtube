use crate::application::http::{self, ApiServices, VideosRoot};
use crate::application::{subscribers, tasks};
use crate::domain::services::{
    ChannelCreator, ChannelDeleter, ChannelPreviewer, ChannelUpdater, ChannelVideoReconciler,
    ChannelViewSearcher, DirectorySearcher, InternalVideoReconciler, MetadataGenerator,
    PlaylistCreator, PlaylistDeleter, PlaylistPreviewer, PlaylistSearcher, PlaylistUpdater,
    PlaylistVideoReconciler, PlexCollectionDeleter, PlexCollectionReconciler, PlexFolderScanner,
    TaskViewSearcher, ThumbnailFetcher, VideoDownloader, VideoFileDeleter, VideoSearcher,
    VideoWatchStateUpdater,
};
use crate::infrastructure::client::ytdlp_updater::{RealYtdlpUpdater, YtdlpUpdater, target_path};
use crate::infrastructure::infrastructure_container::{
    InfrastructureContainer, InfrastructureSettings,
};
use crate::infrastructure::repositories::domain_events_consumer::{
    DomainEventsConsumer, SubscriberRegistry,
};
use crate::infrastructure::repositories::plex_collection_repository::{
    HttpPlexCollectionRepository, PlexCollectionRepository, PlexConfig,
};
use crate::infrastructure::repositories::task_executor::{HandlerRegistry, TaskExecutor};
use crate::infrastructure::shared::web_assets::WebAssets;
use crate::infrastructure::shared::{sqlite_connection, sqlite_migrations};
use anyhow::{Context, Result};
use axum::Router;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
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
const DEFAULT_DOWNLOAD_CONCURRENCY: usize = 2;
const DEFAULT_AVATARS_PATH: &str = "avatars";
const DEFAULT_PLEX_RECONCILE_INTERVAL_SECONDS: i64 = 900;
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

fn download_concurrency() -> usize {
    parse_download_concurrency(std::env::var("YARRTUBE_DOWNLOAD_CONCURRENCY").ok())
}

/// Keeps only a positive integer, defaulting otherwise.
fn parse_download_concurrency(value: Option<String>) -> usize {
    value
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|concurrency| *concurrency > 0)
        .unwrap_or(DEFAULT_DOWNLOAD_CONCURRENCY)
}

fn videos_path() -> String {
    std::env::var("YARRTUBE_VIDEOS_PATH").unwrap_or_else(|_| DEFAULT_VIDEOS_PATH.to_string())
}

/// Lives alongside the SQLite database, not under the videos root. The Docker
/// image sets both to `/config` (see the Dockerfile's `ENV`) so they persist
/// on the same volume; the relative default only applies outside Docker.
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

fn plex_reconcile_interval_seconds() -> i64 {
    std::env::var("YARRTUBE_PLEX_RECONCILE_INTERVAL_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PLEX_RECONCILE_INTERVAL_SECONDS)
}

/// The Plex integration's adapter and target sections, present only when
/// the daemon is configured to talk to a Plex server: `YARRTUBE_PLEX_URL`,
/// `YARRTUBE_PLEX_TOKEN` and at least one of
/// `YARRTUBE_PLEX_PLAYLIST_SECTION_ID` / `YARRTUBE_PLEX_CHANNEL_SECTION_ID`
/// (each one section id, or a comma-separated list for content spread across
/// several libraries) must be set to a non-empty value (wrappers like
/// `run-local.sh` pass empty strings for unset variables). Each section is
/// scoped to a single kind: playlist sections are reconciled against tracked
/// playlists and channel sections against tracked channels. When absent,
/// nothing Plex-related is wired and the daemon behaves exactly as without
/// the integration.
#[derive(Clone)]
struct PlexIntegration {
    repository: Arc<dyn PlexCollectionRepository>,
    playlist_section_ids: Vec<String>,
    channel_section_ids: Vec<String>,
    /// `YARRTUBE_PLEX_VIDEOS_PATH`: where Plex sees `videos_path()`. Without
    /// it, downloaded videos' folders aren't scanned into Plex explicitly.
    plex_videos_path: Option<String>,
}

fn plex_integration() -> Option<PlexIntegration> {
    let base_url = non_empty_env("YARRTUBE_PLEX_URL")?;
    let token = non_empty_env("YARRTUBE_PLEX_TOKEN")?;
    let (playlist_section_ids, channel_section_ids) = plex_section_ids(
        std::env::var("YARRTUBE_PLEX_PLAYLIST_SECTION_ID").ok(),
        std::env::var("YARRTUBE_PLEX_CHANNEL_SECTION_ID").ok(),
    )?;
    Some(PlexIntegration {
        repository: Arc::new(HttpPlexCollectionRepository::new(PlexConfig {
            base_url,
            token,
        })),
        playlist_section_ids,
        channel_section_ids,
        plex_videos_path: non_empty_env("YARRTUBE_PLEX_VIDEOS_PATH"),
    })
}

/// Parses the two kind-scoped section lists, returning `None` when both are
/// empty — the signal that the integration stays off. A present list keeps
/// only its non-empty, trimmed comma-separated entries.
fn plex_section_ids(
    playlist: Option<String>,
    channel: Option<String>,
) -> Option<(Vec<String>, Vec<String>)> {
    let playlist_section_ids = parse_section_ids(playlist);
    let channel_section_ids = parse_section_ids(channel);
    if playlist_section_ids.is_empty() && channel_section_ids.is_empty() {
        return None;
    }
    Some((playlist_section_ids, channel_section_ids))
}

fn parse_section_ids(value: Option<String>) -> Vec<String> {
    value
        .into_iter()
        .flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|section_id| !section_id.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
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
/// previous run left `running` and seeds the recurring yt-dlp self-update
/// and, when the Plex integration is enabled, the recurring Plex
/// collections reconcile.
fn prepare_task_queue(
    infrastructure: &InfrastructureContainer,
    task_executor: &TaskExecutor,
    plex_enabled: bool,
) -> Result<()> {
    task_executor
        .recover_stuck_tasks()
        .context("failed to recover tasks left running from a previous run")?;

    let update_ytdlp_first_run_at = infrastructure.clock.now()
        + chrono::Duration::seconds(tasks::update_ytdlp_task::UPDATE_INTERVAL_SECONDS);
    tasks::update_ytdlp_task::schedule_update_ytdlp_if_absent(
        infrastructure.task_repository.as_ref(),
        update_ytdlp_first_run_at,
    )
    .context("failed to schedule the recurring yt-dlp self-update task")?;

    if plex_enabled {
        tasks::reconcile_plex_collections_task::schedule_reconcile_plex_collections_if_absent(
            &infrastructure.task_repository,
            &infrastructure.clock,
        )
        .context("failed to schedule the recurring Plex collections reconcile task")?;
    }
    Ok(())
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
        playlist_previewer: PlaylistPreviewer::new(
            infrastructure.youtube_playlist_repository.clone(),
        ),
        playlist_searcher: PlaylistSearcher::new(infrastructure.playlist_repository.clone()),
        playlist_updater: PlaylistUpdater::new(infrastructure.playlist_repository.clone()),
        playlist_video_reconciler: playlist_video_reconciler(infrastructure),
        video_searcher: VideoSearcher::new(
            infrastructure.playlist_repository.clone(),
            infrastructure.playlist_video_repository.clone(),
            infrastructure.channel_repository.clone(),
            infrastructure.channel_video_repository.clone(),
            infrastructure.video_repository.clone(),
            infrastructure.video_metadata_repository.clone(),
            infrastructure.clock.clone(),
        ),
        task_view_searcher: TaskViewSearcher::new(
            infrastructure.task_repository.clone(),
            infrastructure.playlist_repository.clone(),
            infrastructure.channel_repository.clone(),
            infrastructure.video_repository.clone(),
            infrastructure.playlist_video_repository.clone(),
            infrastructure.channel_video_repository.clone(),
        ),
        channel_creator: ChannelCreator::new(
            infrastructure.channel_repository.clone(),
            infrastructure.youtube_channel_repository.clone(),
            infrastructure.channel_avatar_repository.clone(),
            infrastructure.event_publisher.clone(),
            infrastructure.clock.clone(),
        ),
        channel_deleter: ChannelDeleter::new(
            infrastructure.channel_repository.clone(),
            infrastructure.video_repository.clone(),
            infrastructure.channel_video_repository.clone(),
            infrastructure.channel_avatar_repository.clone(),
            infrastructure.event_publisher.clone(),
        ),
        channel_previewer: ChannelPreviewer::new(infrastructure.youtube_channel_repository.clone()),
        channel_updater: ChannelUpdater::new(infrastructure.channel_repository.clone()),
        channel_view_searcher: ChannelViewSearcher::new(
            infrastructure.channel_repository.clone(),
            infrastructure.channel_video_repository.clone(),
            infrastructure.video_repository.clone(),
        ),
        channel_video_reconciler: channel_video_reconciler(infrastructure),
        directory_searcher: DirectorySearcher::new(infrastructure.directory_repository.clone()),
        video_watch_state_updater: VideoWatchStateUpdater::new(
            infrastructure.video_repository.clone(),
            infrastructure.channel_repository.clone(),
            infrastructure.channel_video_repository.clone(),
            infrastructure.clock.clone(),
        ),
        videos_root: VideosRoot(videos_path()),
    }
}

fn event_consumer(
    infrastructure: &InfrastructureContainer,
    plex: Option<PlexIntegration>,
) -> DomainEventsConsumer {
    DomainEventsConsumer::new(
        infrastructure.event_repository.clone(),
        event_subscribers(infrastructure, plex),
        infrastructure.clock.clone(),
    )
}

fn event_subscribers(
    infrastructure: &InfrastructureContainer,
    plex: Option<PlexIntegration>,
) -> SubscriberRegistry {
    subscribers::registry(
        playlist_video_reconciler(infrastructure),
        channel_video_reconciler(infrastructure),
        infrastructure.playlist_repository.clone(),
        infrastructure.channel_repository.clone(),
        infrastructure.task_repository.clone(),
        infrastructure.clock.clone(),
        videos_path(),
        plex.clone().map(|plex| {
            PlexCollectionDeleter::new(
                plex.playlist_section_ids,
                plex.channel_section_ids,
                plex.repository,
            )
        }),
        plex.and_then(plex_folder_scanner),
    )
}

/// Present only when `YARRTUBE_PLEX_VIDEOS_PATH` says where Plex sees the
/// videos root; it scans in every configured section, of either kind.
fn plex_folder_scanner(plex: PlexIntegration) -> Option<PlexFolderScanner> {
    let plex_videos_path = plex.plex_videos_path?;
    Some(PlexFolderScanner::new(
        [plex.playlist_section_ids, plex.channel_section_ids].concat(),
        videos_path(),
        plex_videos_path,
        plex.repository,
    ))
}

fn task_executor(
    infrastructure: &InfrastructureContainer,
    plex: Option<PlexIntegration>,
) -> TaskExecutor {
    TaskExecutor::new(
        infrastructure.task_repository.clone(),
        task_handlers(infrastructure, plex),
        infrastructure.clock.clone(),
        retry_base_delay_seconds(),
        download_concurrency(),
    )
}

fn task_handlers(
    infrastructure: &InfrastructureContainer,
    plex: Option<PlexIntegration>,
) -> HandlerRegistry {
    tasks::registry(
        playlist_video_reconciler(infrastructure),
        channel_video_reconciler(infrastructure),
        video_downloader(infrastructure),
        thumbnail_fetcher(infrastructure),
        infrastructure.video_repository.clone(),
        VideoFileDeleter::new(infrastructure.video_file_repository.clone(), videos_path()),
        infrastructure.task_repository.clone(),
        infrastructure.clock.clone(),
        infrastructure.ytdlp_updater.clone(),
        target_path(),
        plex.map(|plex| {
            tasks::reconcile_plex_collections_task::ReconcilePlexCollectionsTask::new(
                plex_collection_reconciler(infrastructure, plex),
                infrastructure.task_repository.clone(),
                infrastructure.clock.clone(),
                plex_reconcile_interval_seconds(),
            )
        }),
    )
}

fn plex_collection_reconciler(
    infrastructure: &InfrastructureContainer,
    plex: PlexIntegration,
) -> PlexCollectionReconciler {
    PlexCollectionReconciler::new(
        plex.playlist_section_ids,
        plex.channel_section_ids,
        infrastructure.playlist_repository.clone(),
        infrastructure.channel_repository.clone(),
        infrastructure.playlist_video_repository.clone(),
        infrastructure.channel_video_repository.clone(),
        infrastructure.video_repository.clone(),
        plex.repository,
    )
}

fn playlist_video_reconciler(infrastructure: &InfrastructureContainer) -> PlaylistVideoReconciler {
    PlaylistVideoReconciler::new(
        infrastructure.playlist_repository.clone(),
        infrastructure.video_repository.clone(),
        infrastructure.playlist_video_repository.clone(),
        infrastructure.youtube_playlist_items_repository.clone(),
        infrastructure.event_publisher.clone(),
        infrastructure.task_repository.clone(),
        internal_video_reconciler(infrastructure),
        infrastructure.clock.clone(),
        reconcile_interval_seconds(),
    )
}

fn channel_video_reconciler(infrastructure: &InfrastructureContainer) -> ChannelVideoReconciler {
    ChannelVideoReconciler::new(
        infrastructure.channel_repository.clone(),
        infrastructure.video_repository.clone(),
        infrastructure.channel_video_repository.clone(),
        infrastructure.channel_videos_repository.clone(),
        infrastructure.event_publisher.clone(),
        infrastructure.task_repository.clone(),
        internal_video_reconciler(infrastructure),
        infrastructure.clock.clone(),
        reconcile_interval_seconds(),
    )
}

fn video_downloader(infrastructure: &InfrastructureContainer) -> VideoDownloader {
    VideoDownloader::new(
        infrastructure.video_repository.clone(),
        infrastructure.video_downloader_repository.clone(),
        infrastructure.video_file_repository.clone(),
        infrastructure.playlist_video_repository.clone(),
        metadata_generator(infrastructure),
        infrastructure.video_metadata_repository.clone(),
        infrastructure.event_publisher.clone(),
        infrastructure.clock.clone(),
    )
}

fn metadata_generator(infrastructure: &InfrastructureContainer) -> Arc<MetadataGenerator> {
    Arc::new(MetadataGenerator::new(
        infrastructure.youtube_metadata_repository.clone(),
        infrastructure.clock.clone(),
    ))
}

fn internal_video_reconciler(
    infrastructure: &InfrastructureContainer,
) -> Arc<InternalVideoReconciler> {
    Arc::new(InternalVideoReconciler::new(
        infrastructure.video_repository.clone(),
        metadata_generator(infrastructure),
        infrastructure.video_metadata_repository.clone(),
        infrastructure.task_repository.clone(),
        infrastructure.video_file_repository.clone(),
        thumbnail_fetcher(infrastructure),
        infrastructure.clock.clone(),
        videos_path(),
    ))
}

fn thumbnail_fetcher(infrastructure: &InfrastructureContainer) -> Arc<ThumbnailFetcher> {
    Arc::new(ThumbnailFetcher::new(
        infrastructure.video_repository.clone(),
        infrastructure.video_downloader_repository.clone(),
        infrastructure.task_repository.clone(),
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
    plex: Option<PlexIntegration>,
) -> ExitCode {
    tokio::spawn(heartbeat_loop());
    tokio::spawn(Arc::new(event_consumer(&infrastructure, plex)).run(BACKGROUND_POLL_INTERVAL));
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
    let plex = plex_integration();
    let task_executor = Arc::new(task_executor(&infrastructure, plex.clone()));
    if let Err(e) = prepare_task_queue(&infrastructure, &task_executor, plex.is_some()) {
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

    runtime.block_on(run_async(infrastructure, task_executor, plex))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use std::collections::{BTreeMap, BTreeSet};
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

    #[test]
    fn it_should_register_a_handler_for_every_task_type() {
        let db = TestDatabase::new();
        let infrastructure = test_infrastructure(&db);

        let handlers = task_handlers(&infrastructure, None);

        assert_eq!(
            handlers.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "delete_channel_files",
                "delete_playlist_files",
                "delete_video_file",
                "download_video",
                "fetch_thumbnail",
                "reconcile_channel",
                "reconcile_playlist",
                "update_ytdlp",
            ])
        );
    }

    #[test]
    fn it_should_register_the_subscribers_of_every_event_type() {
        let db = TestDatabase::new();
        let infrastructure = test_infrastructure(&db);

        let subscribers = event_subscribers(&infrastructure, None);

        assert_eq!(
            subscribers
                .iter()
                .map(|(event_type, subscribers)| (event_type.as_str(), subscribers.len()))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                ("channel_created", 1),
                ("channel_deleted", 1),
                ("playlist_created", 1),
                ("playlist_deleted", 1),
                ("video_added_to_channel", 2),
                ("video_added_to_playlist", 2),
                ("video_removed_from_channel", 1),
                ("video_removed_from_playlist", 1),
            ])
        );
    }

    #[test]
    fn it_should_register_the_plex_subscribers_with_the_plex_videos_path() {
        let db = TestDatabase::new();
        let infrastructure = test_infrastructure(&db);

        let subscribers = event_subscribers(&infrastructure, Some(test_plex(Some("/plex/videos"))));

        assert_eq!(
            subscribers
                .iter()
                .map(|(event_type, subscribers)| (event_type.as_str(), subscribers.len()))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([
                ("channel_created", 1),
                ("channel_deleted", 2),
                ("playlist_created", 1),
                ("playlist_deleted", 2),
                ("video_added_to_channel", 2),
                ("video_added_to_playlist", 2),
                ("video_downloaded", 1),
                ("video_removed_from_channel", 1),
                ("video_removed_from_playlist", 1),
            ])
        );
    }

    #[test]
    fn it_should_not_scan_plex_folders_without_the_plex_videos_path() {
        let db = TestDatabase::new();
        let infrastructure = test_infrastructure(&db);

        let subscribers = event_subscribers(&infrastructure, Some(test_plex(None)));

        assert_eq!(subscribers.get("video_downloaded").map(Vec::len), None);
    }

    #[test]
    fn it_should_disable_plex_when_both_section_lists_are_empty() {
        assert_eq!(plex_section_ids(None, None), None);
        assert_eq!(
            plex_section_ids(Some(String::new()), Some("  ,  ".to_string())),
            None
        );
    }

    #[test]
    fn it_should_enable_plex_with_only_playlist_sections() {
        assert_eq!(
            plex_section_ids(Some("19, 21".to_string()), None),
            Some((vec!["19".to_string(), "21".to_string()], vec![]))
        );
    }

    #[test]
    fn it_should_enable_plex_with_only_channel_sections() {
        assert_eq!(
            plex_section_ids(Some(String::new()), Some("20".to_string())),
            Some((vec![], vec!["20".to_string()]))
        );
    }

    #[test]
    fn it_should_enable_plex_with_both_playlist_and_channel_sections() {
        assert_eq!(
            plex_section_ids(Some("19".to_string()), Some("20".to_string())),
            Some((vec!["19".to_string()], vec!["20".to_string()]))
        );
    }

    #[test]
    fn it_should_default_download_concurrency_when_invalid() {
        let concurrencies: Vec<usize> = [None, Some("0"), Some("-1"), Some("abc"), Some("4")]
            .into_iter()
            .map(|value| parse_download_concurrency(value.map(str::to_string)))
            .collect();

        assert_eq!(concurrencies, vec![2, 2, 2, 2, 4]);
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

    /// Production adapters over a fresh test database, as `run` builds them.
    fn test_infrastructure(db: &TestDatabase) -> InfrastructureContainer {
        InfrastructureContainer::new(InfrastructureSettings {
            db_path: db.path(),
            youtube_api_key: String::new(),
            videos_path: unique_temp_dir("serve-registry-videos"),
            avatars_path: unique_temp_dir("serve-registry-avatars"),
            ytdlp_path: PathBuf::from("yt-dlp"),
        })
        .unwrap()
    }

    fn test_plex(plex_videos_path: Option<&str>) -> PlexIntegration {
        PlexIntegration {
            repository: Arc::new(
                crate::infrastructure::repositories::plex_collection_repository::FakePlexCollectionRepository::default(),
            ),
            playlist_section_ids: vec!["19".to_string()],
            channel_section_ids: vec!["21".to_string()],
            plex_videos_path: plex_videos_path.map(str::to_string),
        }
    }
}
