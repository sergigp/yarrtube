//! The Clock is a port (faked with `FixedClock` in tests): the domain and the
//! adapters that call it never read the wall clock directly. Infrastructure
//! is left out: it implements the clock, and its other `now()` reads make
//! unique temp names, not time.

use syn::visit::Visit;

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, macro_args, path_idents};

pub const RULE: Rule = Rule {
    name: "clock_is_a_port",
    fix: "Take the time from the injected `Clock` port (`clock.now()`), faked with `FixedClock` in \
          tests, instead of reading `Utc::now()` / `Local::now()` / `SystemTime::now()`.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Persistence and Test Doubles: the Clock)",
};

const WALL_CLOCKS: [&str; 3] = ["Utc", "Local", "SystemTime"];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/domain/") || file.is_under("src/application/"))
        .flat_map(|file| {
            let mut reads = WallClockReads::default();
            reads.visit_file(&file.ast);
            reads
                .found
                .into_iter()
                .map(|(read, line)| {
                    Violation::new(&file.path, format!("{}:{line}: reads `{read}`", file.path))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Calls to `Utc::now()`, `Local::now()` and `SystemTime::now()`.
#[derive(Default)]
struct WallClockReads {
    found: Vec<(String, usize)>,
}

impl<'ast> Visit<'ast> for WallClockReads {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(func) = call.func.as_ref() {
            let idents = path_idents(&func.path);
            if let [.., clock, now] = idents.as_slice()
                && now == "now"
                && WALL_CLOCKS.contains(&clock.as_str())
            {
                self.found.push((
                    format!("{clock}::now()"),
                    func.path.segments[0].ident.span().start().line,
                ));
            }
        }
        syn::visit::visit_expr_call(self, call);
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
    fn it_should_report_a_direct_read_of_the_wall_clock() {
        let files = [
            SourceFile::new(
                "src/domain/services/widget_creator.rs",
                "fn created_at() -> DateTime<Utc> { chrono::Utc::now() }",
            ),
            SourceFile::new(
                "src/application/tasks/widget_task/mod.rs",
                "fn stamp() -> SystemTime { std::time::SystemTime::now() }",
            ),
        ];

        assert_eq!(
            units(&violations(&files)),
            vec![
                "src/application/tasks/widget_task/mod.rs",
                "src/domain/services/widget_creator.rs"
            ]
        );
    }

    #[test]
    fn it_should_accept_the_clock_port_and_the_system_clock() {
        let files = [
            SourceFile::new(
                "src/domain/services/widget_creator.rs",
                "fn created_at(&self) { self.clock.now() }\n// not Utc::now()",
            ),
            SourceFile::new(
                "src/infrastructure/shared/system_clock.rs",
                "fn now(&self) -> DateTime<Utc> { Utc::now() }",
            ),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
