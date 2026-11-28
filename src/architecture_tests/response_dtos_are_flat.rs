//! A response DTO never composes another one with `#[serde(flatten)]`.

use syn::Item;

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, type_name};

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
                let Item::Struct(dto) = scoped.item else {
                    return None;
                };
                flattened_dtos(dto).first().map(|flattened| {
                    Violation::new(
                        format!("{}::{}", file.path, dto.ident),
                        format!(
                            "{}:{}: `{}` flattens `{flattened}`",
                            file.path,
                            file.line_of(&dto.ident),
                            dto.ident
                        ),
                    )
                })
            })
        })
        .collect()
}

/// The DTO types of the fields marked `#[serde(flatten)]`.
fn flattened_dtos(dto: &syn::ItemStruct) -> Vec<String> {
    dto.fields
        .iter()
        .filter(|field| field.attrs.iter().any(is_serde_flatten))
        .map(|field| type_name(&field.ty))
        .filter(|ty| ty.ends_with("Response") || ty.ends_with("Dto"))
        .collect()
}

fn is_serde_flatten(attr: &syn::Attribute) -> bool {
    let mut flatten = false;
    if attr.path().is_ident("serde") {
        let _ = attr.parse_nested_meta(|meta| {
            flatten |= meta.path.is_ident("flatten");
            if meta.input.peek(syn::Token![=]) {
                meta.value()?.parse::<syn::Lit>()?;
            }
            Ok(())
        });
    }
    flatten
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
