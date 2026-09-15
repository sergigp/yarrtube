use crate::infrastructure::client::ytdlp_updater::{RealYtdlpUpdater, YtdlpUpdater, target_path};
use std::process::ExitCode;

pub fn run() -> ExitCode {
    let path = target_path();
    match RealYtdlpUpdater.update(&path) {
        Ok(()) => {
            println!("yt-dlp updated successfully at {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: failed to update yt-dlp: {e}");
            ExitCode::FAILURE
        }
    }
}
