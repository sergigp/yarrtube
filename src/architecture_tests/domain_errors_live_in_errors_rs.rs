//! Every error type of an aggregate lives in its `errors.rs`.

use syn::Item;

use super::baseline::{Rule, Violation};
use super::source::SourceFile;

pub const RULE: Rule = Rule {
    name: "domain_errors_live_in_errors_rs",
    fix: "Move the error type into the aggregate's `errors.rs` (src/domain/<aggregate>/errors.rs; \
          `ValidationError` lives in src/domain/shared/errors.rs), next to the aggregate's other \
          error types, not in the file that raises it.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (File Structure: `errors.rs`)",
};

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/domain/") && file.file_name() != "errors.rs")
        .flat_map(|file| {
            file.all_items()
                .into_iter()
                .filter(|s| !s.in_test)
                .filter_map(move |scoped| {
                    let ident = match scoped.item {
                        Item::Struct(s) => &s.ident,
                        Item::Enum(e) => &e.ident,
                        _ => return None,
                    };
                    ident.to_string().ends_with("Error").then(|| {
                        Violation::new(
                            format!("{}::{ident}", file.path),
                            format!(
                                "{}:{}: error type `{ident}` outside errors.rs",
                                file.path,
                                file.line_of(ident)
                            ),
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
    fn it_should_report_an_error_type_declared_outside_errors_rs() {
        let files = [SourceFile::new(
            "src/domain/services/widget_creator.rs",
            "#[derive(Debug)]\npub enum CreateWidgetError {\n    Lookup(anyhow::Error),\n}",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/domain/services/widget_creator.rs::CreateWidgetError"]
        );
    }

    #[test]
    fn it_should_accept_error_types_in_errors_rs() {
        let files = [
            SourceFile::new(
                "src/domain/widget/errors.rs",
                "pub enum CreateWidgetError { Lookup }",
            ),
            SourceFile::new(
                "src/domain/widget/widget.rs",
                "pub struct Widget;\n#[cfg(test)]\nmod tests {\n    struct FakeError;\n}",
            ),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
