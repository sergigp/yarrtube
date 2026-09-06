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
        /// URL of the YouTube playlist to download
        playlist_url: String,

        /// Local directory to download videos into
        output_path: String,
    },

    /// Run the long-lived daemon: HTTP server, startup checks, heartbeat
    Serve,

    /// Download the latest yt-dlp release and replace the local binary
    UpdateYtdlp,
}
