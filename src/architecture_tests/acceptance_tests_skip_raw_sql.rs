//! Acceptance and behaviour tests seed and assert through the port traits,
//! never through raw SQL.

use syn::visit::Visit;
use syn::{Expr, Lit};

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, macro_args};

pub const RULE: Rule = Rule {
    name: "acceptance_tests_skip_raw_sql",
    fix: "Seed and assert through the repository's port (`insert`, `list`, `find`, …). When an \
          assertion needs a read the port lacks, add a `#[cfg(test)]` inherent method on the SQLite \
          implementation instead of running SQL from the test.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Persistence and Test Doubles)",
};

const SQL_CALLS: [&str; 5] = [
    "execute",
    "execute_batch",
    "prepare",
    "prepare_cached",
    "query_row",
];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/application/") || file.is_under("src/domain/services/"))
        .filter_map(|file| {
            let mut sql = Sql::default();
            file.test_items()
                .into_iter()
                .for_each(|item| sql.visit_item(item));
            sql.found.first().map(|(call, line)| {
                Violation::new(
                    &file.path,
                    format!("{}:{line}: its tests run SQL (`.{call}(\"…\")`)", file.path),
                )
            })
        })
        .collect()
}

/// Calls like `.execute("INSERT …")`: a SQL method whose first argument is a
/// string literal.
#[derive(Default)]
struct Sql {
    found: Vec<(String, usize)>,
}

impl<'ast> Visit<'ast> for Sql {
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let method = call.method.to_string();
        let sql_literal =
            matches!(call.args.first(), Some(Expr::Lit(lit)) if matches!(lit.lit, Lit::Str(_)));
        if SQL_CALLS.contains(&method.as_str()) && sql_literal {
            self.found.push((method, call.method.span().start().line));
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        macro_args(node).iter().for_each(|arg| self.visit_expr(arg));
        syn::visit::visit_macro(self, node);
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
