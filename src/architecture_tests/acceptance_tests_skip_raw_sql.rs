//! Acceptance and behaviour tests seed and assert through the port traits,
//! never through raw SQL.

use super::baseline::{Rule, Violation};
use super::source::SourceFile;

pub const RULE: Rule = Rule {
    name: "acceptance_tests_skip_raw_sql",
    fix: "Seed and assert through the repository's port (`insert`, `list`, `find`, …). When an \
          assertion needs a read the port lacks, add a `#[cfg(test)]` inherent method on the SQLite \
          implementation instead of running SQL from the test.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Persistence and Test Doubles)",
};

const SQL_CALLS: [&str; 6] = [
    ".execute(\"",
    ".execute(r",
    ".execute_batch(",
    ".prepare(\"",
    ".prepare(r",
    ".query_row(\"",
];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/application/") || file.is_under("src/domain/services/"))
        .filter_map(|file| {
            let code: String = file.test_code().concat().split_whitespace().collect();
            SQL_CALLS
                .iter()
                .find(|call| code.contains(*call))
                .map(|call| {
                    Violation::new(
                        &file.path,
                        format!("{}: its tests run SQL (`{call}…`)", file.path),
                    )
                })
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
    fn it_should_report_a_test_seeding_with_sql() {
        let files = [SourceFile::new(
            "src/application/tasks/widget_task/tests.rs",
            "#[test]\nfn it_should_run() {\n    db.connection()\n        .execute(\n            \"INSERT INTO widgets VALUES (1)\",\n            [],\n        );\n}\n",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/application/tasks/widget_task/tests.rs"]
        );
    }

    #[test]
    fn it_should_accept_sql_in_production_code_and_port_calls_in_tests() {
        let files = [SourceFile::new(
            "src/domain/services/widget_summarizer.rs",
            "fn run(&self) { self.prepare(video); }\n#[cfg(test)]\nmod tests {\n    \
             fn seed() { repository.insert(&widget()).unwrap(); }\n}\n",
        )];

        assert_eq!(violations(&files), vec![]);
    }
}
