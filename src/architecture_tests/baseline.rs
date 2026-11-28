//! The baseline of a rule: the units (a file, or a symbol inside one) that
//! already broke it the day it arrived, listed in `baselines/<rule>.txt`. The
//! rule ignores them, and the baseline only shrinks: a listed unit that no
//! longer breaks the rule fails the check until its line is deleted.

use std::collections::BTreeSet;
use std::path::Path;

pub struct Rule {
    /// The rule's name, also its baseline file name.
    pub name: &'static str,
    /// What to do instead, for the agent reading the failure.
    pub fix: &'static str,
    /// Where the convention is written: `Convention: <path> (<section>)`.
    pub convention: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Violation {
    /// The smallest unit the baseline names: `src/x.rs` or `src/x.rs::Symbol`.
    pub unit: String,
    /// Where it happens and what it is, for the failure message.
    pub detail: String,
}

impl Violation {
    pub fn new(unit: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            unit: unit.into(),
            detail: detail.into(),
        }
    }
}

/// Fails the test when the repository breaks `rule` outside its baseline, or
/// when a unit of the baseline no longer breaks it.
pub fn assert_matches_baseline(rule: &Rule, violations: Vec<Violation>) {
    if let Err(message) = compare(rule, &read_baseline(rule.name), &violations) {
        panic!("{message}");
    }
}

/// The unit names of some violations, deduplicated and sorted.
pub fn units(violations: &[Violation]) -> Vec<String> {
    violations
        .iter()
        .map(|v| v.unit.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn compare(
    rule: &Rule,
    baseline: &BTreeSet<String>,
    violations: &[Violation],
) -> Result<(), String> {
    let found: BTreeSet<String> = violations.iter().map(|v| v.unit.clone()).collect();
    let new: Vec<String> = violations
        .iter()
        .filter(|v| !baseline.contains(&v.unit))
        .map(|v| format!("  - {}", v.detail))
        .collect();
    let fixed: Vec<String> = baseline
        .difference(&found)
        .map(|unit| format!("  - {unit}"))
        .collect();
    let mut problems = Vec::new();
    if !new.is_empty() {
        problems.push(format!(
            "rule `{}` is broken:\n{}\n{}\n{}",
            rule.name,
            new.join("\n"),
            rule.fix,
            rule.convention
        ));
    }
    if !fixed.is_empty() {
        problems.push(format!(
            "these units of the baseline of rule `{}` no longer break it: delete their lines from \
             src/architecture_tests/baselines/{}.txt (a baseline only shrinks):\n{}",
            rule.name,
            rule.name,
            fixed.join("\n")
        ));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n\n"))
    }
}

fn read_baseline(rule: &str) -> BTreeSet<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/architecture_tests/baselines")
        .join(format!("{rule}.txt"));
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULE: Rule = Rule {
        name: "some_rule",
        fix: "Do it the other way.",
        convention: "Convention: doc/SOMEWHERE.md",
    };

    #[test]
    fn it_should_accept_violations_listed_in_the_baseline() {
        let baseline = BTreeSet::from(["src/a.rs".to_string()]);

        assert_eq!(
            compare(
                &RULE,
                &baseline,
                &[Violation::new("src/a.rs", "src/a.rs:1: x")]
            ),
            Ok(())
        );
    }

    #[test]
    fn it_should_fail_on_a_violation_outside_the_baseline() {
        let result = compare(
            &RULE,
            &BTreeSet::new(),
            &[Violation::new("src/a.rs", "src/a.rs:1: x")],
        );

        assert_eq!(
            result,
            Err("rule `some_rule` is broken:\n  - src/a.rs:1: x\nDo it the other way.\nConvention: doc/SOMEWHERE.md"
                .to_string())
        );
    }

    #[test]
    fn it_should_fail_when_a_baseline_unit_no_longer_breaks_the_rule() {
        let baseline = BTreeSet::from(["src/a.rs".to_string()]);

        assert_eq!(
            compare(&RULE, &baseline, &[]),
            Err("these units of the baseline of rule `some_rule` no longer break it: delete their lines from \
                 src/architecture_tests/baselines/some_rule.txt (a baseline only shrinks):\n  - src/a.rs"
                .to_string())
        );
    }
}
