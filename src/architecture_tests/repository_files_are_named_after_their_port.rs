//! src/infrastructure/repositories/ holds port implementations named
//! `<implementation>_<port>.rs`.

use super::baseline::{Rule, Violation};
use super::source::SourceFile;

pub const RULE: Rule = Rule {
    name: "repository_files_are_named_after_their_port",
    fix: "Name a file in src/infrastructure/repositories/ `<implementation>_<port>.rs` (e.g. \
          `sqlite_playlist_repository.rs`), holding the port trait and its implementation. Infra \
          that is not one aggregate's port goes in src/infrastructure/shared/ or client/.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (File Structure: `infrastructure/repositories/`)",
};

const DIR: &str = "src/infrastructure/repositories/";

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under(DIR) && !file.path[DIR.len()..].contains('/'))
        .filter(|file| {
            file.file_name() != "mod.rs" && !file.file_name().ends_with("_repository.rs")
        })
        .map(|file| {
            Violation::new(
                &file.path,
                format!("{}: not named `<implementation>_<port>.rs`", file.path),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::baseline::{assert_matches_baseline, units};
    use super::super::source::repository_sources;
    use super::*;

    #[test]
    fn it_should_hold_on_the_repository() {
        assert_matches_baseline(&RULE, violations(repository_sources()));
    }

    #[test]
    fn it_should_report_a_file_not_named_after_a_port() {
        let files = [SourceFile::new(
            "src/infrastructure/repositories/widget_helpers.rs",
            "",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/infrastructure/repositories/widget_helpers.rs"]
        );
    }

    #[test]
    fn it_should_accept_files_named_after_their_port() {
        let files = [
            SourceFile::new(
                "src/infrastructure/repositories/sqlite_widget_repository.rs",
                "",
            ),
            SourceFile::new("src/infrastructure/repositories/mod.rs", ""),
            SourceFile::new("src/infrastructure/shared/system_clock.rs", ""),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
