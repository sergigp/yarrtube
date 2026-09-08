pub mod download_command;
pub mod ytdlp_update;

use crate::domain::shared::PlaylistId;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "yarrtube")]
#[command(about = "Download YouTube playlists via yt-dlp, and run as a long-lived daemon")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Download every video in a YouTube playlist
    Download {
        /// YouTube playlist ID to download
        playlist_id: PlaylistId,

        /// Local directory to download videos into
        output_path: String,
    },

    /// Run the long-lived daemon: HTTP server, startup checks, heartbeat
    Serve,

    /// Download the latest yt-dlp release and replace the local binary
    UpdateYtdlp,
}
