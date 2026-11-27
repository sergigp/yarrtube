//! Tests are named `it_should_<behaviour>[_if_<condition>]`.

use super::baseline::{Rule, Violation};
use super::source::{ItemKind, SourceFile};

pub const RULE: Rule = Rule {
    name: "test_naming",
    fix: "Name the test `it_should_<behaviour>[_if_<condition>]`: the behaviour, not the mechanics \
          (`it_should_create_a_channel`, `it_should_fail_if_invalid_handle_provided`, \
          `it_should_skip_if_playlist_is_gone`).",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Testing Opinions: test naming)",
};

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            file.all_items()
                .into_iter()
                .filter_map(move |scoped| match &scoped.item.kind {
                    ItemKind::Fn { name, .. }
                        if scoped.item.is_test_fn() && !name.starts_with("it_should_") =>
                    {
                        Some(Violation::new(
                            format!("{}::{name}", file.path),
                            format!(
                                "{}:{}: test `{name}`",
                                file.path,
                                file.line_of(scoped.item.start)
                            ),
                        ))
                    }
                    _ => None,
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
    fn it_should_report_tests_not_named_it_should() {
        let files = [SourceFile::new(
            "src/application/http/widgets/tests.rs",
            "#[test]\nfn creates_a_widget() {}\n#[tokio::test]\nasync fn test_delete() {}\n",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec![
                "src/application/http/widgets/tests.rs::creates_a_widget",
                "src/application/http/widgets/tests.rs::test_delete",
            ]
        );
    }

    #[test]
    fn it_should_accept_it_should_tests_and_any_helper_name() {
        let files = [SourceFile::new(
            "src/domain/widget/widget.rs",
            "#[cfg(test)]\nmod tests {\n    #[test]\n    fn it_should_create_a_widget() {}\n    fn a_widget() {}\n}\n",
        )];

        assert_eq!(violations(&files), vec![]);
    }
}
