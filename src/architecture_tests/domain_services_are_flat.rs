//! Every use-case service sits directly in src/domain/services/.

use super::baseline::{Rule, Violation};
use super::source::SourceFile;

pub const RULE: Rule = Rule {
    name: "domain_services_are_flat",
    fix: "Put the service file directly in src/domain/services/<use_case>.rs: services stay \
          together and flat, with no subfolders.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (File Structure: `services/`)",
};

const DIR: &str = "src/domain/services/";

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under(DIR))
        .filter_map(|file| {
            file.path[DIR.len()..].split_once('/').map(|(folder, _)| {
                Violation::new(
                    format!("{DIR}{folder}/"),
                    format!("{}: inside a subfolder of {DIR}", file.path),
                )
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
    fn it_should_report_a_service_in_a_subfolder() {
        let files = [SourceFile::new(
            "src/domain/services/widgets/widget_creator.rs",
            "",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/domain/services/widgets/"]
        );
    }

    #[test]
    fn it_should_accept_services_directly_in_the_folder() {
        let files = [SourceFile::new("src/domain/services/widget_creator.rs", "")];

        assert_eq!(violations(&files), vec![]);
    }
}
