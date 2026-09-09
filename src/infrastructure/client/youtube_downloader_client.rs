use crate::domain::video::VideoFilename;
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

            let filename = VideoFilename::from_title(&video.title);
            let size_before = dir_size(output_path);
            let start = Instant::now();
            let result = download_video(&video.url, filename.as_str(), &video.id, output_path)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::ytdlp::test_support::{FakeYtDlpOnPath, unique_temp_dir};

    #[test]
    #[cfg(unix)]
    fn it_should_download_each_video_using_its_sanitized_title_as_the_filename() {
        let output_dir = unique_temp_dir("youtube-downloader-client");
        let guard = FakeYtDlpOnPath::with_exit_code(0);

        let messy_title = "My: Messy / Title?";
        let videos = vec![Video {
            id: "vid1".into(),
            url: "https://www.youtube.com/watch?v=vid1".into(),
            title: messy_title.into(),
        }];

        let summary = YtDlpDownloaderClient
            .download_all(&videos, &output_dir)
            .unwrap();

        assert_eq!(summary.succeeded, 1);
        let expected_filename = VideoFilename::from_title(messy_title);
        assert_eq!(
            guard.captured_args(),
            vec![
                videos[0].url.clone(),
                "-o".to_string(),
                format!("{}.%(ext)s", expected_filename.as_str()),
            ]
        );
        std::fs::remove_dir_all(&output_dir).unwrap();
    }
}
