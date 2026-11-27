//! An acceptance test module reads: imports, then every `it_should_*` test,
//! then the helpers.

use super::baseline::{Rule, Violation};
use super::source::{Item, ItemKind, SourceFile};

pub const RULE: Rule = Rule {
    name: "acceptance_test_module_layout",
    fix: "Lay the test module out as: every `use` first, then every `#[test]`/`#[tokio::test]` fn, \
          then the helpers (builders, fakes, consts, other fns) at the bottom.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Acceptance Tests: module layout)",
};

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/application/"))
        .flat_map(|file| {
            file.test_modules()
                .into_iter()
                .filter_map(move |(module, items)| {
                    misplaced(&items).map(|item| {
                        Violation::new(
                            format!("{}::{module}", file.path),
                            format!(
                                "{}:{}: `{}` is out of place (imports, then tests, then helpers)",
                                file.path,
                                file.line_of(item.start),
                                item.signature
                            ),
                        )
                    })
                })
        })
        .collect()
}

/// The first item that comes after a later group's item.
fn misplaced(items: &[Item]) -> Option<&Item> {
    let group = |item: &Item| match item.kind {
        ItemKind::Use => 0,
        _ if item.is_test_fn() => 1,
        _ => 2,
    };
    // Constants go anywhere: the test modules keep them at the top as often
    // as among the helpers.
    let is_constant = |item: &&Item| {
        item.signature.starts_with("const ") || item.signature.starts_with("static ")
    };
    let items: Vec<&Item> = items.iter().filter(|item| !is_constant(item)).collect();
    items
        .windows(2)
        .find(|pair| group(pair[1]) < group(pair[0]))
        .map(|pair| pair[1])
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
    fn it_should_report_a_test_after_a_helper() {
        let files = [SourceFile::new(
            "src/application/http/widgets/tests.rs",
            "use super::*;\n#[tokio::test]\nasync fn it_should_create() {}\nfn a_request() {}\n\
             #[tokio::test]\nasync fn it_should_delete() {}\n",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/application/http/widgets/tests.rs::tests"]
        );
    }

    #[test]
    fn it_should_accept_imports_and_constants_then_tests_then_helpers() {
        let files = [SourceFile::new(
            "src/application/subscribers/widget_subscriber.rs",
            "pub struct S;\n#[cfg(test)]\nmod tests {\n    use super::*;\n    const NAME: &str = \"x\";\n    #[test]\n    fn it_should_work() {}\n    \
             struct FakeThing;\n    const OTHER: &str = \"y\";\n    fn a_request() {}\n}\n",
        )];

        assert_eq!(violations(&files), vec![]);
    }
}
