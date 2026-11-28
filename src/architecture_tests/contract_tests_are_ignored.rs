//! Tests against the real YouTube / Anthropic live in `youtube_contract_tests`
//! / `anthropic_contract_tests` modules and are `#[ignore]`d: `cargo test`
//! skips them and `scripts/run-tests.sh --contract` runs them by name.

use syn::Item;

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, has_attribute, is_test_fn};

pub const RULE: Rule = Rule {
    name: "contract_tests_are_ignored",
    fix: "Put a test against the real YouTube/yt-dlp in a `mod youtube_contract_tests` (the Anthropic \
          one in `mod anthropic_contract_tests`) next to the adapter, and mark it `#[ignore]`. \
          `scripts/run-tests.sh --contract` runs exactly those; any other `#[ignore]`d test never runs.",
    convention: "Convention: doc/DEVELOPMENT.md (Contract tests)",
};

const CONTRACT_MODULES: [&str; 2] = ["youtube_contract_tests", "anthropic_contract_tests"];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            file.all_items().into_iter().filter_map(move |scoped| {
                let Item::Fn(f) = scoped.item else {
                    return None;
                };
                let in_contract_module = scoped.modules.iter().any(|m| CONTRACT_MODULES.contains(&m.as_str()));
                let ignored = has_attribute(&f.attrs, "ignore");
                let problem = match (is_test_fn(scoped.item), in_contract_module, ignored) {
                    (true, true, false) => "contract test not `#[ignore]`d: plain `cargo test` would hit the real service",
                    (true, false, true) => "`#[ignore]`d test outside a contract module: no suite runs it",
                    _ => return None,
                };
                let name = &f.sig.ident;
                Some(Violation::new(
                    format!("{}::{name}", file.path),
                    format!("{}:{}: `{name}`: {problem}", file.path, file.line_of(name)),
                ))
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
    fn it_should_report_unignored_contract_tests_and_stray_ignored_tests() {
        let files = [SourceFile::new(
            "src/infrastructure/repositories/youtube_widget_repository.rs",
            "#[cfg(test)]\nmod tests {\n    #[test]\n    #[ignore]\n    fn it_should_be_slow() {}\n}\n\
             #[cfg(test)]\nmod youtube_contract_tests {\n    #[test]\n    fn it_should_list_real_widgets() {}\n}\n",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec![
                "src/infrastructure/repositories/youtube_widget_repository.rs::it_should_be_slow",
                "src/infrastructure/repositories/youtube_widget_repository.rs::it_should_list_real_widgets",
            ]
        );
    }

    #[test]
    fn it_should_accept_ignored_tests_in_contract_modules() {
        let files = [SourceFile::new(
            "src/infrastructure/repositories/youtube_widget_repository.rs",
            "#[cfg(test)]\nmod youtube_contract_tests {\n    #[test]\n    #[ignore = \"needs YouTube\"]\n    \
             fn it_should_list_real_widgets() {}\n    fn helper() {}\n}\n",
        )];

        assert_eq!(violations(&files), vec![]);
    }
}
