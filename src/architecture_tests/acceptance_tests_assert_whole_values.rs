//! Acceptance and behaviour tests assert whole typed values, never `len()`.

use syn::visit::Visit;

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, macro_args, path_idents};

pub const RULE: Rule = Rule {
    name: "acceptance_tests_assert_whole_values",
    fix: "Assert the whole value in one `assert_eq!` (`repository.list().unwrap() == vec![..]`), never \
          its `len()`.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Acceptance Tests: assert whole typed values)",
};

const ASSERTS: [&str; 3] = ["assert", "assert_eq", "assert_ne"];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/application/") || file.is_under("src/domain/services/"))
        .filter_map(|file| {
            let mut asserts = LenAsserts::default();
            file.test_items()
                .into_iter()
                .for_each(|item| asserts.visit_item(item));
            asserts.lines.first().map(|line| {
                Violation::new(
                    &file.path,
                    format!("{}:{line}: its tests assert on `.len()`", file.path),
                )
            })
        })
        .collect()
}

/// The asserts whose arguments call `.len()`.
#[derive(Default)]
struct LenAsserts {
    lines: Vec<usize>,
}

impl<'ast> Visit<'ast> for LenAsserts {
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let args = macro_args(node);
        let is_assert = path_idents(&node.path)
            .last()
            .is_some_and(|name| ASSERTS.contains(&name.as_str()));
        if is_assert && args.iter().any(calls_len) {
            self.lines
                .push(node.path.segments[0].ident.span().start().line);
        }
        args.iter().for_each(|arg| self.visit_expr(arg));
        syn::visit::visit_macro(self, node);
    }
}

fn calls_len(expr: &syn::Expr) -> bool {
    #[derive(Default)]
    struct Len(bool);
    impl<'ast> Visit<'ast> for Len {
        fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
            self.0 |= call.method == "len" && call.args.is_empty();
            syn::visit::visit_expr_method_call(self, call);
        }
    }
    let mut len = Len::default();
    len.visit_expr(expr);
    len.0
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
    fn it_should_report_a_test_asserting_a_length() {
        let files = [SourceFile::new(
            "src/application/subscribers/widget_subscriber.rs",
            "#[cfg(test)]\nmod tests {\n    #[test]\n    fn it_should_add() {\n        \
             assert_eq!(\n            repository.list().unwrap().len(),\n            2\n        );\n    }\n}\n",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/application/subscribers/widget_subscriber.rs"]
        );
    }

    #[test]
    fn it_should_accept_whole_value_asserts_and_len_outside_asserts() {
        let files = [SourceFile::new(
            "src/application/http/widgets/tests.rs",
            "#[tokio::test]\nasync fn it_should_list() {\n    let n = widgets.len();\n    \
             assert_eq!(repository.list().unwrap(), vec![a_widget()]);\n}\n",
        )];

        assert_eq!(violations(&files), vec![]);
    }
}
