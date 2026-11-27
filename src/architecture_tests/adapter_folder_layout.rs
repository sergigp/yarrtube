//! An HTTP resource folder holds `mod.rs`, `dto.rs` and `tests.rs`; a task
//! folder holds `mod.rs` and `tests.rs`.

use super::baseline::{Rule, Violation};
use super::source::SourceFile;

pub const RULE: Rule = Rule {
    name: "adapter_folder_layout",
    fix: "Keep src/application/http/<resource>/ to `mod.rs` (handlers), `dto.rs` (wire types + \
          `From<Domain>`) and `tests.rs`, and src/application/tasks/<task>/ to `mod.rs` and `tests.rs`. \
          Move other code into those files, or into the layer it belongs to.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (File Structure: `application/`)",
};

const FOLDERS: [(&str, &[&str]); 2] = [
    ("src/application/http/", &["mod.rs", "dto.rs", "tests.rs"]),
    ("src/application/tasks/", &["mod.rs", "tests.rs"]),
];

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter_map(|file| {
            FOLDERS.iter().find_map(|(dir, allowed)| {
                let inside = file.path.strip_prefix(dir)?;
                let (folder, rest) = inside.split_once('/')?;
                (!allowed.contains(&rest)).then(|| {
                    Violation::new(
                        &file.path,
                        format!(
                            "{}: `{dir}{folder}/` only holds {}",
                            file.path,
                            allowed.join(", ")
                        ),
                    )
                })
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
    fn it_should_report_extra_files_in_adapter_folders() {
        let files = [
            SourceFile::new("src/application/http/widgets/mapping.rs", ""),
            SourceFile::new("src/application/tasks/widget_task/dto.rs", ""),
        ];

        assert_eq!(
            units(&violations(&files)),
            vec![
                "src/application/http/widgets/mapping.rs",
                "src/application/tasks/widget_task/dto.rs"
            ]
        );
    }

    #[test]
    fn it_should_accept_the_listed_files_and_shared_files_beside_the_folders() {
        let files = [
            SourceFile::new("src/application/http/widgets/mod.rs", ""),
            SourceFile::new("src/application/http/widgets/dto.rs", ""),
            SourceFile::new("src/application/http/widgets/tests.rs", ""),
            SourceFile::new("src/application/http/validation.rs", ""),
            SourceFile::new("src/application/tasks/widget_task/mod.rs", ""),
            SourceFile::new("src/application/tasks/log_capture.rs", ""),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
