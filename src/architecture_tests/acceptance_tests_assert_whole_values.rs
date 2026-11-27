//! Acceptance and behaviour tests assert whole typed values, never `len()`.

use super::baseline::{Rule, Violation};
use super::source::SourceFile;

pub const RULE: Rule = Rule {
    name: "acceptance_tests_assert_whole_values",
    fix: "Assert the whole value in one `assert_eq!` (`repository.list().unwrap() == vec![..]`), never \
          its `len()`.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Acceptance Tests: assert whole typed values)",
};

const ASSERTS: [&str; 3] = ["assert!(", "assert_eq!(", "assert_ne!("];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/application/") || file.is_under("src/domain/services/"))
        .filter(|file| file.test_code().iter().any(|code| asserts_on_len(code)))
        .map(|file| {
            Violation::new(
                &file.path,
                format!("{}: its tests assert on `.len()`", file.path),
            )
        })
        .collect()
}

fn asserts_on_len(code: &str) -> bool {
    ASSERTS.iter().any(|assert| {
        code.match_indices(assert).any(|(at, _)| {
            let open = at + assert.len() - 1;
            arguments(code, open).contains(".len()")
        })
    })
}

/// The text inside the parentheses opening at `open`.
fn arguments(code: &str, open: usize) -> &str {
    let mut depth = 0;
    code[open..]
        .char_indices()
        .find_map(|(i, c)| {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
            (depth == 0).then(|| &code[open + 1..open + i])
        })
        .unwrap_or("")
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
