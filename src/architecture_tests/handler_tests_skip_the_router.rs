//! HTTP acceptance tests call the handler function directly, not through a `Router`.

use syn::visit::Visit;

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, macro_args};

pub const RULE: Rule = Rule {
    name: "handler_tests_skip_the_router",
    fix: "Call the handler function directly with only the service it uses (a small helper per \
          handler unwraps the `Json`). Route wiring is covered by the Playwright suite in e2e-tests/.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Acceptance Tests: HTTP)",
};

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/application/http/"))
        .flat_map(|file| {
            let mut routing = Routing::default();
            file.test_items()
                .into_iter()
                .for_each(|item| routing.visit_item(item));
            routing
                .found
                .into_iter()
                .map(|(what, line)| {
                    Violation::new(
                        &file.path,
                        format!("{}:{line}: its tests use `{what}`", file.path),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Uses of `Router`, `ServiceExt` and `.oneshot(…)`.
#[derive(Default)]
struct Routing {
    found: Vec<(String, usize)>,
}

impl<'ast> Visit<'ast> for Routing {
    fn visit_ident(&mut self, ident: &'ast proc_macro2::Ident) {
        if ident == "Router" || ident == "ServiceExt" {
            self.found
                .push((ident.to_string(), ident.span().start().line));
        }
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if call.method == "oneshot" {
            self.found
                .push(("oneshot".to_string(), call.method.span().start().line));
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
