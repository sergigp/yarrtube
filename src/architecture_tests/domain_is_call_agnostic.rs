//! Domain services know nothing about HTTP, CLI, subscribers or tasks.

use syn::visit::Visit;

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, macro_args, path_idents};

pub const RULE: Rule = Rule {
    name: "domain_is_call_agnostic",
    fix: "Keep src/domain/ free of its callers: no `crate::application`, `axum`, `clap`, `tower` or \
          `tower_http`. Parse and validate the request in src/application/, build value objects \
          there and pass them to the domain service.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (\"Domain services are call-agnostic\")",
};

const CALLERS: [&str; 4] = ["axum", "clap", "tower", "tower_http"];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/domain/"))
        .flat_map(|file| {
            let mut paths = Paths::default();
            paths.visit_file(&file.ast);
            paths
                .found
                .into_iter()
                .map(|(caller, line)| {
                    Violation::new(&file.path, format!("{}:{line}: uses `{caller}`", file.path))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The paths (in `use` declarations and in code) that start at a caller.
#[derive(Default)]
struct Paths {
    found: Vec<(String, usize)>,
}

impl<'ast> Visit<'ast> for Paths {
    fn visit_use_path(&mut self, node: &'ast syn::UsePath) {
        self.check(
            &[node.ident.to_string(), use_next(&node.tree)],
            node.ident.span().start().line,
        );
        syn::visit::visit_use_path(self, node);
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        let idents = path_idents(node);
        if idents.len() > 1 {
            self.check(&idents, node.segments[0].ident.span().start().line);
        }
        syn::visit::visit_path(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        macro_args(node).iter().for_each(|arg| self.visit_expr(arg));
        syn::visit::visit_macro(self, node);
    }
}

impl Paths {
    fn check(&mut self, idents: &[String], line: usize) {
        let caller = match idents {
            [first, second, ..] if first == "crate" && second == "application" => {
                "crate::application".to_string()
            }
            [first, ..] if CALLERS.contains(&first.as_str()) => format!("{first}::"),
            _ => return,
        };
        if !self.found.contains(&(caller.clone(), line)) {
            self.found.push((caller, line));
        }
    }
}

/// The identifier after `crate::` in a `use crate::…` tree.
fn use_next(tree: &syn::UseTree) -> String {
    match tree {
        syn::UseTree::Path(path) => path.ident.to_string(),
        syn::UseTree::Name(name) => name.ident.to_string(),
        _ => String::new(),
    }
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
