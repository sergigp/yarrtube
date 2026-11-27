//! Domain services know nothing about HTTP, CLI, subscribers or tasks.

use super::baseline::{Rule, Violation};
use super::source::SourceFile;

pub const RULE: Rule = Rule {
    name: "domain_is_call_agnostic",
    fix: "Keep src/domain/ free of its callers: no `crate::application`, `axum`, `clap`, `tower` or \
          `tower_http`. Parse and validate the request in src/application/, build value objects \
          there and pass them to the domain service.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (\"Domain services are call-agnostic\")",
};

const CALLERS: [&str; 5] = [
    "crate::application",
    "axum::",
    "clap::",
    "tower::",
    "tower_http::",
];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/domain/"))
        .flat_map(|file| {
            CALLERS.iter().flat_map(move |caller| {
                file.token_offsets(caller).into_iter().map(move |offset| {
                    Violation::new(
                        &file.path,
                        format!("{}:{}: uses `{caller}`", file.path, file.line_of(offset)),
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
    fn it_should_report_a_domain_file_using_an_application_adapter() {
        let files = [
            SourceFile::new(
                "src/domain/services/a.rs",
                "use crate::application::http::ApiError;",
            ),
            SourceFile::new(
                "src/domain/services/b.rs",
                "fn f() -> axum::http::StatusCode { todo() }",
            ),
        ];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/domain/services/a.rs", "src/domain/services/b.rs"]
        );
    }

    #[test]
    fn it_should_accept_adapters_using_the_domain() {
        let files = [
            SourceFile::new(
                "src/application/http/a.rs",
                "use axum::Json; use crate::domain::x;",
            ),
            SourceFile::new(
                "src/domain/services/b.rs",
                "// axum::Json is not used here\nuse crate::infrastructure::x;",
            ),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
