//! A response DTO never composes another one with `#[serde(flatten)]`.

use super::baseline::{Rule, Violation};
use super::source::{ItemKind, SourceFile, squash};

pub const RULE: Rule = Rule {
    name: "response_dtos_are_flat",
    fix: "Give the response DTO the flat fields its consumers use instead of `#[serde(flatten)]`-ing \
          another response DTO into it.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (HTTP Handlers: response DTOs)",
};

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| file.is_under("src/application/http/"))
        .flat_map(|file| {
            file.all_items().into_iter().filter_map(move |scoped| {
                let ItemKind::Struct { name } = &scoped.item.kind else {
                    return None;
                };
                let body = squash(file.body_of(&scoped.item));
                flattened_dtos(&body).first().map(|dto| {
                    Violation::new(
                        format!("{}::{name}", file.path),
                        format!(
                            "{}:{}: `{name}` flattens `{dto}`",
                            file.path,
                            file.line_of(scoped.item.start)
                        ),
                    )
                })
            })
        })
        .collect()
}

/// The DTO types of the fields marked `#[serde(flatten)]` in a struct body.
fn flattened_dtos(body: &str) -> Vec<String> {
    body.split("#[serde(flatten)]")
        .skip(1)
        .filter_map(|after| {
            let field = after.split(',').next()?;
            let ty = field.split_once(':')?.1.trim();
            let base: String = ty
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (base.ends_with("Response") || base.ends_with("Dto")).then_some(base)
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
    fn it_should_report_a_response_flattening_another() {
        let files = [SourceFile::new(
            "src/application/http/widgets/dto.rs",
            "#[derive(Serialize)]\npub struct WidgetDetailResponse {\n    #[serde(flatten)]\n    \
             pub widget: WidgetResponse,\n    pub extra: u32,\n}\n",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/application/http/widgets/dto.rs::WidgetDetailResponse"]
        );
    }

    #[test]
    fn it_should_accept_flat_responses_and_genuinely_nested_ones() {
        let files = [SourceFile::new(
            "src/application/http/widgets/dto.rs",
            "#[derive(Serialize)]\npub struct WidgetResponse {\n    pub id: String,\n    pub source: SourceResponse,\n}\n",
        )];

        assert_eq!(violations(&files), vec![]);
    }
}
