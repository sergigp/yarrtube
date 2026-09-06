use crate::downloader;
use crate::youtube_api;
use std::path::Path;
use std::process::ExitCode;

pub fn run(playlist_url: &str, output_path: &str) -> ExitCode {
    let api_key = match std::env::var("YOUTUBE_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            eprintln!(
                "Error: YOUTUBE_API_KEY is not set. Copy .env.example to .env and fill in your YouTube Data API v3 key."
            );
            return ExitCode::FAILURE;
        }
    };

    let playlist_id = match youtube_api::extract_playlist_id(playlist_url) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let videos = match youtube_api::resolve_playlist(&playlist_id, &api_key) {
        Ok(videos) => videos,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if videos.is_empty() {
        println!("No videos found in this playlist.");
        return ExitCode::SUCCESS;
    }

    let output_path = Path::new(output_path);
    let summary = match downloader::download_all(&videos, output_path) {
        Ok(summary) => summary,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!();
    println!(
        "Done: {} succeeded, {} failed out of {}",
        summary.succeeded,
        summary.failed.len(),
        videos.len()
    );

    if summary.failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        println!("Failed videos:");
        for video in &summary.failed {
            println!("  - {} ({})", video.title, video.url);
        }
        ExitCode::FAILURE
    }
}
