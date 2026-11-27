//! A change's tasks.md is a vertical slice: its sections start with the
//! four openspec/config.yaml names, in order.

use super::baseline::{Rule, Violation};
use super::openspec::Artifact;

pub const RULE: Rule = Rule {
    name: "tasks_md_sections",
    fix: "Start tasks.md with exactly these sections, in order: `## 1. Walking skeleton`, \
          `## 2. Behaviour (TDD)`, `## 3. Infrastructure adapters (TDD)`, `## 4. Verification`. \
          Never group tasks by layer.",
    convention: "Convention: openspec/config.yaml (rules.tasks)",
};

const SECTIONS: [&str; 4] = [
    "1. Walking skeleton",
    "2. Behaviour (TDD)",
    "3. Infrastructure adapters (TDD)",
    "4. Verification",
];

pub fn violations(artifacts: &[Artifact]) -> Vec<Violation> {
    artifacts
        .iter()
        .filter(|tasks| !tasks.sections().starts_with(&SECTIONS))
        .map(|tasks| {
            Violation::new(
                &tasks.path,
                format!(
                    "{}: sections {:?} don't start with {SECTIONS:?}",
                    tasks.path,
                    tasks.sections()
                ),
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
        assert_matches_baseline(&RULE, violations(&active_artifacts("tasks.md")));
    }

    #[test]
    fn it_should_report_tasks_grouped_by_layer() {
        let artifacts = [Artifact::new(
            "openspec/changes/a/tasks.md",
            "## 1. Domain\n\n- [ ] 1.1 x\n\n## 2. Infrastructure\n\n## 3. Application\n",
        )];

        assert_eq!(
            units(&violations(&artifacts)),
            vec!["openspec/changes/a/tasks.md"]
        );
    }

    #[test]
    fn it_should_accept_the_four_sections_in_order() {
        let artifacts = [Artifact::new(
            "openspec/changes/a/tasks.md",
            "## 1. Walking skeleton\n\n## 2. Behaviour (TDD)\n\n## 3. Infrastructure adapters (TDD)\n\n## 4. Verification\n",
        )];

        assert_eq!(violations(&artifacts), vec![]);
    }
}
