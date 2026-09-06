use crate::ytdlp_update;
use anyhow::{Context, Result};
use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_DB_PATH: &str = "yarrtube.sqlite3";
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

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

async fn serve_http(port: u16) -> Result<()> {
    let app = Router::new().route("/status", get(status));
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .with_context(|| format!("failed to bind HTTP server on port {port}"))?;
    println!("[startup] HTTP server listening on 0.0.0.0:{port}");
    axum::serve(listener, app)
        .await
        .context("HTTP server failed")?;
    Ok(())
}

async fn run_async() -> ExitCode {
    tokio::spawn(heartbeat_loop());

    if let Err(e) = serve_http(port()).await {
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

    runtime.block_on(run_async())
}
