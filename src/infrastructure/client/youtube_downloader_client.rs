use crate::infrastructure::shared::youtube_api::Video;
use anyhow::{Result, anyhow};
use std::io;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub struct DownloadSummary {
    pub succeeded: usize,
    pub failed: Vec<Video>,
}

pub trait YoutubeDownloaderClient: Send + Sync {
    fn download_all(&self, videos: &[Video], output_path: &Path) -> Result<DownloadSummary>;
}

pub struct YtDlpDownloaderClient;

impl YoutubeDownloaderClient for YtDlpDownloaderClient {
    fn download_all(&self, videos: &[Video], output_path: &Path) -> Result<DownloadSummary> {
        ensure_output_dir(output_path)?;

        let mut succeeded = 0;
        let mut failed = Vec::new();

        for video in videos {
            println!("Downloading: {} ({})", video.title, video.url);

            let size_before = dir_size(output_path);
            let start = Instant::now();
            let result = download_video(&video.url, output_path)?;
            let elapsed = start.elapsed().as_secs_f64().max(0.001);
            let downloaded_bytes = dir_size(output_path).saturating_sub(size_before);
            let speed = downloaded_bytes as f64 / elapsed;
            println!(
                "  took {elapsed:.1}s, size={}, avg speed={}/s",
                format_bytes(downloaded_bytes),
                format_bytes(speed as u64)
            );

            match result {
                true => succeeded += 1,
                false => {
                    eprintln!("Failed to download: {} ({})", video.title, video.url);
                    failed.push(video.clone());
                }
            }
        }

        Ok(DownloadSummary { succeeded, failed })
    }
}

fn ensure_output_dir(output_path: &Path) -> Result<()> {
    std::fs::create_dir_all(output_path)
        .map_err(|e| anyhow!("Failed to create output directory {output_path:?}: {e}"))
}

/// Runs `yt-dlp <video_url>` in `output_path`. Returns `Ok(true)`/`Ok(false)` for a
/// completed process based on its exit status. Returns `Err` only when `yt-dlp`
/// itself could not be spawned (e.g. not found on `PATH`) — a systemic setup
/// problem, distinct from a single video failing to download.
fn download_video(video_url: &str, output_path: &Path) -> Result<bool> {
    println!("Running: yt-dlp {video_url} (in {})", output_path.display());
    match Command::new("yt-dlp")
        .arg(video_url)
        .current_dir(output_path)
        .status()
    {
        Ok(status) => Ok(status.success()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(anyhow!(
            "`yt-dlp` was not found on PATH. Install yt-dlp and make sure it is available before running yarrtube."
        )),
        Err(e) => Err(anyhow!("Failed to run yt-dlp for {video_url}: {e}")),
    }
}

fn dir_size(path: &Path) -> u64 {
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.metadata().ok())
                .filter(|meta| meta.is_file())
                .map(|meta| meta.len())
                .sum()
        })
        .unwrap_or(0)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{size:.2} {}", UNITS[unit])
}
