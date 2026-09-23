use super::error::ApiError;

/// Runs a synchronous service call on tokio's blocking thread pool and awaits
/// its result, so the I/O inside it (database access, the blocking YouTube
/// HTTP client) never stalls an async worker thread. Every handler routes its
/// service call through here, so none has to judge whether a service is slow
/// enough to need it. A task that panics or is cancelled is reported as a
/// `500 Internal Server Error`.
pub async fn run_blocking<T, F>(f: F) -> Result<T, ApiError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(ApiError::internal)
}
