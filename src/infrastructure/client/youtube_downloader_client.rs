use crate::infrastructure::shared::youtube_api::Video;
use crate::infrastructure::shared::ytdlp::{download_video, ensure_output_dir};
use anyhow::Result;
use std::path::Path;
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
