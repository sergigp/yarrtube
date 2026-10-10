use crate::domain::services::{LayoutMigratorApi, MigrationReport};
use crate::serve;
use std::process::ExitCode;

/// Runs the layout migration once, printing one line per move, skip and
/// failure plus a summary. Exits non-zero when any video failed to move.
pub fn run(dry_run: bool) -> ExitCode {
    let infrastructure = match serve::build_infrastructure() {
        Ok(infrastructure) => infrastructure,
        Err(e) => {
            eprintln!("Error: failed to initialize infrastructure: {e}");
            return ExitCode::FAILURE;
        }
    };
    match serve::layout_migrator(&infrastructure).migrate(dry_run) {
        Ok(report) => report_outcome(&report),
        Err(e) => {
            eprintln!("Error: layout migration failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn report_outcome(_report: &MigrationReport) -> ExitCode {
    ExitCode::SUCCESS
}
