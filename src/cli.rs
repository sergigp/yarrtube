use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "yarrtube")]
#[command(about = "Download every video in a YouTube playlist via yt-dlp")]
pub struct Cli {
    /// URL of the YouTube playlist to download
    pub playlist_url: String,

    /// Local directory to download videos into
    pub output_path: String,
}
