pub mod migrate_layout;
pub mod update_ytdlp;

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
    /// Run the long-lived daemon: HTTP server, startup checks, heartbeat
    Serve,

    /// Download the latest yt-dlp release and replace the local binary
    UpdateYtdlp,

    /// Move an existing library into the TV-show layout (stop the daemon first)
    MigrateLayout {
        /// Print what would move without changing anything
        #[arg(long)]
        dry_run: bool,
    },
}
