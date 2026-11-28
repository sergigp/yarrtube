//! HTTP handler and task acceptance tests live in a sibling `tests.rs`,
//! declared with `#[cfg(test)] mod tests;` at the end of `mod.rs`.

use std::collections::BTreeSet;

use syn::Item;

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, is_cfg_test};

pub const RULE: Rule = Rule {
    name: "adapter_tests_live_in_sibling_file",
    fix: "Put the handlers' (or the task's) acceptance tests in the sibling `tests.rs` and end \
          `mod.rs` with `#[cfg(test)] mod tests;`, instead of an inline `mod tests { … }`.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Infrastructure Tests: sibling `tests.rs`)",
};

const DIRS: [&str; 2] = ["src/application/http/", "src/application/tasks/"];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    let paths: BTreeSet<&str> = files.iter().map(|file| file.path.as_str()).collect();
    files
        .iter()
        .filter(|file| {
            file.file_name() == "mod.rs"
                && DIRS
                    .iter()
                    .any(|dir| file.path.strip_prefix(dir).is_some_and(|rest| rest.matches('/').count() == 1))
        })
        .filter_map(|file| {
            let folder = file.path.trim_end_matches("mod.rs");
            let ends_with_tests_decl = matches!(
                file.ast.items.last(),
                Some(Item::Mod(m)) if m.ident == "tests" && m.content.is_none() && is_cfg_test(&m.attrs)
            );
            let problem = if !ends_with_tests_decl {
                "doesn't end with `#[cfg(test)] mod tests;`"
            } else if !paths.contains(format!("{folder}tests.rs").as_str()) {
                "has no sibling `tests.rs`"
            } else {
                return None;
            };
            Some(Violation::new(&file.path, format!("{}: {problem}", file.path)))
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
    fn it_should_report_inline_tests_and_a_missing_tests_file() {
        let files = [
            SourceFile::new(
                "src/application/http/widgets/mod.rs",
                "pub async fn create() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn it_should_create() {}\n}\n",
            ),
            SourceFile::new(
                "src/application/tasks/widget_task/mod.rs",
                "pub fn handle() {}\n#[cfg(test)]\nmod tests;\n",
            ),
        ];

        assert_eq!(
            units(&violations(&files)),
            vec![
                "src/application/http/widgets/mod.rs",
                "src/application/tasks/widget_task/mod.rs"
            ]
        );
    }

    #[test]
    fn it_should_accept_a_mod_rs_declaring_its_sibling_tests() {
        let files = [
            SourceFile::new(
                "src/application/http/widgets/mod.rs",
                "pub async fn create() {}\n\n#[cfg(test)]\nmod tests;\n",
            ),
            SourceFile::new("src/application/http/widgets/tests.rs", ""),
            SourceFile::new("src/application/http/mod.rs", "pub fn router() {}"),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
