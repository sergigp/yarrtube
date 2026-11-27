//! design.md's `## Review` ends with the line pointing the human at the tests.

use super::baseline::{Rule, Violation};
use super::openspec::Artifact;

pub const RULE: Rule = Rule {
    name: "design_review_ends_with_check",
    fix: "End `## Review` with the line: \"Check: the ## Test Plan test names are the behaviour \
          contract; review those and the final /audit-change result, not the prose below.\"",
    convention: "Convention: openspec/config.yaml (rules.design: ## Review)",
};

const CHECK: &str = "Check: the ## Test Plan test names are the behaviour contract";

pub fn violations(artifacts: &[Artifact]) -> Vec<Violation> {
    artifacts
        .iter()
        .filter(|design| design.sections().contains(&"Review"))
        .filter(|design| {
            let last = design
                .section("Review")
                .into_iter()
                .rev()
                .find(|line| !line.trim().is_empty());
            !last.is_some_and(|line| line.trim().trim_start_matches("- ").starts_with(CHECK))
        })
        .map(|design| {
            Violation::new(
                &design.path,
                format!("{}: `## Review` doesn't end with \"{CHECK}…\"", design.path),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::baseline::{assert_matches_baseline, units};
    use super::super::openspec::active_artifacts;
    use super::*;

    #[test]
    fn it_should_hold_on_the_changes_in_progress() {
        assert_matches_baseline(&RULE, violations(&active_artifacts("design.md")));
    }

    #[test]
    fn it_should_report_a_review_without_the_check_line_last() {
        let artifacts = [Artifact::new(
            "openspec/changes/a/design.md",
            "## Review\n\n- Check: the ## Test Plan test names are the behaviour contract; …\n- Out of scope: x\n\n## Test Plan\n",
        )];

        assert_eq!(
            units(&violations(&artifacts)),
            vec!["openspec/changes/a/design.md"]
        );
    }

    #[test]
    fn it_should_accept_a_review_ending_with_the_check_line() {
        let artifacts = [Artifact::new(
            "openspec/changes/a/design.md",
            "## Review\n\n- X, not Y.\n- Check: the ## Test Plan test names are the behaviour contract; review those \
             and the final /audit-change result, not the prose below.\n\n## Test Plan\n",
        )];

        assert_eq!(violations(&artifacts), vec![]);
    }
}
