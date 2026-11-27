//! A port method returns `anyhow::Result<…>`: no custom infrastructure error types.

use super::baseline::{Rule, Violation};
use super::source::{ItemKind, SourceFile};

pub const RULE: Rule = Rule {
    name: "repository_ports_return_anyhow_results",
    fix: "Return `anyhow::Result<Entity>` / `anyhow::Result<Vec<Entity>>` / \
          `anyhow::Result<Option<Entity>>` from a read and `anyhow::Result<()>` from a write. No \
          custom infrastructure error types: the domain service gives a failure its meaning.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (File Structure: repository/port methods; Deliberate deviations)",
};

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/infrastructure/"))
        .flat_map(|file| {
            file.all_items()
                .into_iter()
                .filter(|scoped| !scoped.in_test)
                .filter_map(|scoped| match &scoped.item.kind {
                    ItemKind::Trait { name } if name.ends_with("Repository") => {
                        Some((name.clone(), scoped.item))
                    }
                    _ => None,
                })
                .flat_map(move |(port, item)| {
                    file.children(&item)
                        .into_iter()
                        .filter_map(move |method| match &method.kind {
                            ItemKind::Fn { name, .. }
                                if !is_anyhow_result(&method.return_type()) =>
                            {
                                Some(Violation::new(
                                    format!("{}::{port}::{name}", file.path),
                                    format!(
                                        "{}:{}: `{port}::{name}` returns `{}`",
                                        file.path,
                                        file.line_of(method.start),
                                        method.return_type()
                                    ),
                                ))
                            }
                            _ => None,
                        })
                })
        })
        .collect()
}

fn is_anyhow_result(return_type: &str) -> bool {
    return_type.starts_with("anyhow::Result<")
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
    fn it_should_report_a_port_method_with_a_custom_error_or_no_result() {
        let files = [SourceFile::new(
            "src/infrastructure/repositories/sqlite_widget_repository.rs",
            "pub trait WidgetRepository: Send + Sync {\n    \
             fn find(&self, id: &WidgetId) -> Result<Option<Widget>, LookupError>;\n    \
             fn count(&self) -> usize;\n    \
             fn save(&self, widget: &Widget) -> anyhow::Result<()>;\n}",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec![
                "src/infrastructure/repositories/sqlite_widget_repository.rs::WidgetRepository::count",
                "src/infrastructure/repositories/sqlite_widget_repository.rs::WidgetRepository::find",
            ]
        );
    }

    #[test]
    fn it_should_accept_port_methods_returning_anyhow_results() {
        let files = [SourceFile::new(
            "src/infrastructure/repositories/sqlite_widget_repository.rs",
            "pub trait WidgetRepository: Send + Sync {\n    \
             fn find(&self, id: &WidgetId) -> anyhow::Result<Option<Widget>>;\n    \
             fn list(&self) -> anyhow::Result<Vec<Widget>>;\n}\n\
             pub trait Clock {\n    fn now(&self) -> DateTime<Utc>;\n}",
        )];

        assert_eq!(violations(&files), vec![]);
    }
}
