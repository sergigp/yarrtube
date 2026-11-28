//! Deliberate deviation: a port's trait lives next to its implementation in
//! src/infrastructure/, not in src/domain/.

use syn::Item;

use super::baseline::{Rule, Violation};
use super::source::SourceFile;

pub const RULE: Rule = Rule {
    name: "port_traits_live_in_infrastructure",
    fix: "Move the `…Repository` trait next to its implementation in \
          src/infrastructure/repositories/<implementation>_<port>.rs (or infrastructure/shared/ \
          when several aggregates use it). This is a deliberate deviation: don't move ports into \
          the domain.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Infrastructure Layer; Deliberate deviations)",
};

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/domain/"))
        .flat_map(|file| {
            file.all_items()
                .into_iter()
                .filter_map(move |scoped| match scoped.item {
                    Item::Trait(t) if t.ident.to_string().ends_with("Repository") => {
                        Some(Violation::new(
                            format!("{}::{}", file.path, t.ident),
                            format!(
                                "{}:{}: port trait `{}` in the domain",
                                file.path,
                                file.line_of(&t.ident),
                                t.ident
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
    fn it_should_report_a_port_trait_declared_in_the_domain() {
        let files = [SourceFile::new(
            "src/domain/widget/widget.rs",
            "pub trait WidgetRepository: Send + Sync {\n    fn find(&self) -> anyhow::Result<()>;\n}",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/domain/widget/widget.rs::WidgetRepository"]
        );
    }

    #[test]
    fn it_should_accept_a_port_trait_next_to_its_implementation() {
        let files = [
            SourceFile::new(
                "src/infrastructure/repositories/sqlite_widget_repository.rs",
                "pub trait WidgetRepository: Send + Sync {}",
            ),
            SourceFile::new(
                "src/domain/services/widget_creator.rs",
                "pub trait WidgetCreatorApi: Send + Sync {}",
            ),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
