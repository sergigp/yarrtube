//! HTTP acceptance tests call the handler function directly, not through a `Router`.

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, squash};

pub const RULE: Rule = Rule {
    name: "handler_tests_skip_the_router",
    fix: "Call the handler function directly with only the service it uses (a small helper per \
          handler unwraps the `Json`). Route wiring is covered by the Playwright suite in e2e-tests/.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Acceptance Tests: HTTP)",
};

const ROUTING: [&str; 3] = ["Router", "oneshot(", "ServiceExt"];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/application/http/"))
        .flat_map(|file| {
            file.test_code().into_iter().flat_map(move |code| {
                let code = squash(code);
                ROUTING
                    .iter()
                    .filter(|token| code.contains(*token))
                    .map(|token| {
                        Violation::new(
                            &file.path,
                            format!("{}: its tests use `{token}`", file.path),
                        )
                    })
                    .collect::<Vec<_>>()
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
    fn it_should_report_a_handler_test_going_through_a_router() {
        let files = [SourceFile::new(
            "src/application/http/widgets/tests.rs",
            "use tower::ServiceExt;\n#[tokio::test]\nasync fn it_should_create() {\n    \
             let response = router().oneshot(request).await;\n}\n",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/application/http/widgets/tests.rs"]
        );
    }

    #[test]
    fn it_should_accept_tests_calling_the_handler_and_routers_in_production_code() {
        let files = [
            SourceFile::new(
                "src/application/http/widgets/tests.rs",
                "#[tokio::test]\nasync fn it_should_create() {\n    let r = create(service, request).await;\n}\n",
            ),
            SourceFile::new(
                "src/application/http/mod.rs",
                "pub fn router() -> Router { Router::new() }",
            ),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
