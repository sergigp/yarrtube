//! The Clock is a port (faked with `FixedClock` in tests): the domain and the
//! adapters that call it never read the wall clock directly. Infrastructure
//! is left out: it implements the clock, and its other `now()` reads make
//! unique temp names, not time.

use super::baseline::{Rule, Violation};
use super::source::SourceFile;

pub const RULE: Rule = Rule {
    name: "clock_is_a_port",
    fix: "Take the time from the injected `Clock` port (`clock.now()`), faked with `FixedClock` in \
          tests, instead of reading `Utc::now()` / `Local::now()` / `SystemTime::now()`.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Persistence and Test Doubles: the Clock)",
};

const WALL_CLOCK_READS: [&str; 3] = ["Utc::now()", "Local::now()", "SystemTime::now()"];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/domain/") || file.is_under("src/application/"))
        .flat_map(|file| {
            WALL_CLOCK_READS.iter().flat_map(move |read| {
                file.code.match_indices(read).map(move |(at, _)| {
                    Violation::new(
                        &file.path,
                        format!("{}:{}: reads `{read}`", file.path, file.line_of(at)),
                    )
                })
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
    fn it_should_report_a_direct_read_of_the_wall_clock() {
        let files = [
            SourceFile::new(
                "src/domain/services/widget_creator.rs",
                "fn created_at() -> DateTime<Utc> { chrono::Utc::now() }",
            ),
            SourceFile::new(
                "src/application/tasks/widget_task/mod.rs",
                "fn stamp() -> SystemTime { std::time::SystemTime::now() }",
            ),
        ];

        assert_eq!(
            units(&violations(&files)),
            vec![
                "src/application/tasks/widget_task/mod.rs",
                "src/domain/services/widget_creator.rs"
            ]
        );
    }

    #[test]
    fn it_should_accept_the_clock_port_and_the_system_clock() {
        let files = [
            SourceFile::new(
                "src/domain/services/widget_creator.rs",
                "fn created_at(&self) { self.clock.now() }\n// not Utc::now()",
            ),
            SourceFile::new(
                "src/infrastructure/shared/system_clock.rs",
                "fn now(&self) -> DateTime<Utc> { Utc::now() }",
            ),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
