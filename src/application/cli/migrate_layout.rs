use crate::domain::services::{LayoutMigratorApi, MigrationReport, SkipReason};
use crate::serve;
use std::io::Write;
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
        Ok(report) => report_outcome(&report, &mut std::io::stdout()),
        Err(e) => {
            eprintln!("Error: layout migration failed: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Prints one line per move, skip and failure, then a summary; fails when
/// any video failed to move.
fn report_outcome(report: &MigrationReport, output: &mut impl Write) -> ExitCode {
    let moves = report
        .moved
        .iter()
        .map(|(from, to)| format!("moved {from} -> {to}"));
    let skips = report
        .skipped
        .iter()
        .map(|(path, reason)| format!("skipped {path}: {}", skip_reason(*reason)));
    let failures = report
        .failed
        .iter()
        .map(|(path, error)| format!("failed {path}: {error}"));
    let summary = format!(
        "{} moved, {} skipped, {} failed",
        report.moved.len(),
        report.skipped.len(),
        report.failed.len()
    );
    let printed = moves
        .chain(skips)
        .chain(failures)
        .chain(std::iter::once(summary))
        .try_for_each(|line| writeln!(output, "{line}"));
    if printed.is_err() || !report.failed.is_empty() {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn skip_reason(reason: SkipReason) -> &'static str {
    match reason {
        SkipReason::NoPublishTimestamp => "no metadata, so no publish time",
        SkipReason::FileMissing => "file missing",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_exit_non_zero_when_a_move_failed() {
        let report = MigrationReport {
            moved: vec![(
                "playlists/kids/Alpha/Alpha.mp4".to_string(),
                "playlists/kids/Season 2026/S2026E031501 - Alpha.mp4".to_string(),
            )],
            skipped: vec![(
                "playlists/kids/Beta/Beta.mp4".to_string(),
                SkipReason::NoPublishTimestamp,
            )],
            failed: vec![(
                "playlists/kids/Gamma/Gamma.mp4".to_string(),
                "permission denied".to_string(),
            )],
        };
        let mut output = Vec::new();

        let exit_code = report_outcome(&report, &mut output);

        assert_eq!(exit_code, ExitCode::FAILURE);
        assert_eq!(
            String::from_utf8(output).unwrap(),
            concat!(
                "moved playlists/kids/Alpha/Alpha.mp4 -> playlists/kids/Season 2026/S2026E031501 - Alpha.mp4\n",
                "skipped playlists/kids/Beta/Beta.mp4: no metadata, so no publish time\n",
                "failed playlists/kids/Gamma/Gamma.mp4: permission denied\n",
                "1 moved, 1 skipped, 1 failed\n",
            )
        );
    }
}
