mod cli;
mod domain;
mod http;
mod infrastructure;
mod serve;
mod subscribers;
mod tasks;

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
        } => cli::download_command::run(&playlist_id, &output_path),
        Commands::Serve => serve::run(),
        Commands::UpdateYtdlp => cli::ytdlp_update::run(),
    }
}
