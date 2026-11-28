//! A handler extracts only the service(s) it uses, never the whole `ApiServices`.

use syn::visit::Visit;

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, type_name};

pub const RULE: Rule = Rule {
    name: "handlers_extract_only_their_services",
    fix: "Extract only the service the handler uses (`State(service): State<PlaylistCreator>`); \
          `ApiServices` derives `FromRef`, so axum resolves the sub-state.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (HTTP Handlers)",
};

const DIR: &str = "src/application/http/";

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under(DIR) && file.path[DIR.len()..].contains('/'))
        .flat_map(|file| {
            let mut state = WholeState::default();
            state.visit_file(&file.ast);
            state
                .lines
                .into_iter()
                .map(|line| {
                    Violation::new(
                        &file.path,
                        format!("{}:{line}: extracts `State<ApiServices>`", file.path),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Where `State<ApiServices>` appears.
#[derive(Default)]
struct WholeState {
    lines: Vec<usize>,
}

impl<'ast> Visit<'ast> for WholeState {
    fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
        if segment.ident == "State"
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && args.args.iter().any(|arg| matches!(arg, syn::GenericArgument::Type(ty) if type_name(ty) == "ApiServices"))
        {
            self.lines.push(segment.ident.span().start().line);
        }
        syn::visit::visit_path_segment(self, segment);
    }
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
    fn it_should_report_a_handler_extracting_every_service() {
        let files = [SourceFile::new(
            "src/application/http/widgets/mod.rs",
            "pub async fn create(State(services): State<ApiServices>) -> Result<StatusCode, ApiError> {}",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/application/http/widgets/mod.rs"]
        );
    }

    #[test]
    fn it_should_accept_a_handler_extracting_its_service() {
        let files = [
            SourceFile::new(
                "src/application/http/widgets/mod.rs",
                "pub async fn create(State(creator): State<WidgetCreator>) -> Result<StatusCode, ApiError> {}",
            ),
            SourceFile::new(
                "src/application/http/mod.rs",
                "fn router(state: ApiServices) {}",
            ),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
