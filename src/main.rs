mod application;
mod domain;
mod infrastructure;
mod serve;

use application::cli::{Cli, Commands};
use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    dotenvy::dotenv().ok();

    match cli.command {
        Commands::Serve => serve::run(),
        Commands::UpdateYtdlp => application::cli::update_ytdlp::run(),
    }
}
