//! DTO mapping is covered through handler responses, not standalone DTO tests.

use syn::Item;

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, is_cfg_test, is_test_fn};

pub const RULE: Rule = Rule {
    name: "dtos_have_no_tests",
    fix: "Cover the DTO mapping through the handler's acceptance tests (assert the whole response), \
          not with tests in the DTO file.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Acceptance Tests: DTO mapping)",
};

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/application/http/") && is_dto_file(file))
        .filter(|file| {
            file.all_items().iter().any(|scoped| {
                is_test_fn(scoped.item)
                    || matches!(scoped.item, Item::Mod(m) if is_cfg_test(&m.attrs))
            })
        })
        .map(|file| Violation::new(&file.path, format!("{}: has tests", file.path)))
        .collect()
}

fn is_dto_file(file: &SourceFile) -> bool {
    file.file_name() == "dto.rs" || file.file_name().ends_with("_dto.rs")
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
    fn it_should_report_a_dto_file_with_tests() {
        let files = [SourceFile::new(
            "src/application/http/widgets/dto.rs",
            "pub struct WidgetResponse;\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn it_should_map() {}\n}\n",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/application/http/widgets/dto.rs"]
        );
    }

    #[test]
    fn it_should_accept_dto_files_without_tests() {
        let files = [
            SourceFile::new(
                "src/application/http/widgets/dto.rs",
                "pub struct WidgetResponse;",
            ),
            SourceFile::new(
                "src/application/http/widgets/tests.rs",
                "#[test]\nfn it_should_map() {}",
            ),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
