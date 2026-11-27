//! A change's design.md starts with `## Review` and holds only the sections
//! openspec/config.yaml lists.

use super::baseline::{Rule, Violation};
use super::openspec::Artifact;

pub const RULE: Rule = Rule {
    name: "design_md_sections",
    fix: "Give design.md only these sections, `## Review` first: Review, Diagrams, Types & Signatures, \
          Call Stack, Test Plan. Context, decisions and rationale go in `## Review`, one line each.",
    convention: "Convention: openspec/config.yaml (rules.design)",
};

const SECTIONS: [&str; 5] = [
    "Review",
    "Diagrams",
    "Types & Signatures",
    "Call Stack",
    "Test Plan",
];

pub fn violations(artifacts: &[Artifact]) -> Vec<Violation> {
    artifacts
        .iter()
        .flat_map(|design| {
            let sections = design.sections();
            let first = (sections.first() != Some(&"Review"))
                .then(|| "the first section is not `## Review`".to_string());
            let extra = sections
                .iter()
                .filter(|section| !SECTIONS.contains(section))
                .map(|section| format!("`## {section}` is not one of its sections"));
            first
                .into_iter()
                .chain(extra)
                .map(|problem| Violation::new(&design.path, format!("{}: {problem}", design.path)))
                .collect::<Vec<_>>()
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
    fn it_should_report_a_design_without_review_first_or_with_extra_sections() {
        let artifacts = [
            Artifact::new(
                "openspec/changes/a/design.md",
                "## Context\n\nx\n\n## Review\n\n- y\n",
            ),
            Artifact::new(
                "openspec/changes/b/design.md",
                "## Review\n\n- y\n\n## Decisions\n\n- z\n",
            ),
        ];

        assert_eq!(
            units(&violations(&artifacts)),
            vec![
                "openspec/changes/a/design.md",
                "openspec/changes/b/design.md"
            ]
        );
    }

    #[test]
    fn it_should_accept_a_design_with_review_first_and_listed_sections() {
        let artifacts = [Artifact::new(
            "openspec/changes/a/design.md",
            "# Design\n\n## Review\n\n- x\n\n## Types & Signatures\n\n```rust\n```\n\n## Test Plan\n\n1. it_should_x\n",
        )];

        assert_eq!(violations(&artifacts), vec![]);
    }
}
