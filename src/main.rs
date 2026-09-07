mod cli;
mod domain;
mod download_command;
mod downloader;
mod http;
mod infra;
mod serve;
mod youtube_api;
mod ytdlp_update;

use clap::Parser;
use cli::{Cli, Commands};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    dotenvy::dotenv().ok();

    match cli.command {
        Commands::Download {
            playlist_id,
            output_path,
        } => download_command::run(&playlist_id, &output_path),
        Commands::Serve => serve::run(),
        Commands::UpdateYtdlp => ytdlp_update::run(),
    }
}
