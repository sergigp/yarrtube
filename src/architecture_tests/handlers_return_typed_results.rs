//! Handlers return typed results, never an opaque `Response`.

use super::baseline::{Rule, Violation};
use super::source::{ItemKind, SourceFile};

pub const RULE: Rule = Rule {
    name: "handlers_return_typed_results",
    fix: "Return `Result<(StatusCode, Json<T>), ApiError>` when the status varies, \
          `Result<Json<T>, ApiError>` for a plain 200 or `Result<StatusCode, ApiError>` for a bodiless \
          response, never `Response` / `impl IntoResponse`.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (HTTP Handlers)",
};

const DIR: &str = "src/application/http/";

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| {
            file.is_under(DIR)
                && file.path[DIR.len()..].contains('/')
                && file.file_name() == "mod.rs"
        })
        .flat_map(|file| {
            file.items()
                .into_iter()
                .filter_map(move |item| match &item.kind {
                    ItemKind::Fn { name, .. } if is_opaque(&item.return_type()) => {
                        Some(Violation::new(
                            format!("{}::{name}", file.path),
                            format!(
                                "{}:{}: `{name}` returns `{}`",
                                file.path,
                                file.line_of(item.start),
                                item.return_type()
                            ),
                        ))
                    }
                    _ => None,
                })
        })
        .collect()
}

fn is_opaque(return_type: &str) -> bool {
    return_type.contains("IntoResponse") || return_type.rsplit("::").next() == Some("Response")
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
    fn it_should_report_handlers_returning_opaque_responses() {
        let files = [SourceFile::new(
            "src/application/http/widgets/mod.rs",
            "pub async fn create(State(c): State<WidgetCreator>) -> impl IntoResponse {}\n\
             pub async fn delete(State(d): State<WidgetDeleter>) -> axum::response::Response {}\n",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec![
                "src/application/http/widgets/mod.rs::create",
                "src/application/http/widgets/mod.rs::delete"
            ]
        );
    }

    #[test]
    fn it_should_accept_handlers_returning_typed_results() {
        let files = [SourceFile::new(
            "src/application/http/widgets/mod.rs",
            "pub async fn create(State(c): State<WidgetCreator>) -> Result<(StatusCode, Json<WidgetResponse>), ApiError> {}\n\
             pub async fn version() -> Json<VersionResponse> {}\n",
        )];

        assert_eq!(violations(&files), vec![]);
    }
}
