//! A domain service file reads contract first: the struct and an `impl`
//! with only `new`, the `pub trait <Service>Api: Send + Sync`, its `impl`,
//! then one `impl` of private helpers.

use syn::{Item, Visibility};

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, describe, impl_fns, path_idents, type_name};

pub const RULE: Rule = Rule {
    name: "domain_service_layout",
    fix: "Lay the service out in four blocks: (1) `pub struct <Service>` + `impl <Service>` with only \
          `new`; (2) `pub trait <Service>Api: Send + Sync` with every public operation; (3) `impl \
          <Service>Api for <Service>`; (4) one more `impl <Service>` with the private helpers, never \
          `pub`. Supporting types and constants go above the struct; no free fns.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Domain service layout)",
};

const DIR: &str = "src/domain/services/";

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| {
            file.is_under(DIR)
                && !file.path[DIR.len()..].contains('/')
                && file.file_name() != "mod.rs"
        })
        .flat_map(|file| {
            problems(file)
                .into_iter()
                .map(|problem| Violation::new(&file.path, format!("{}: {problem}", file.path)))
        })
        .collect()
}

fn problems(file: &SourceFile) -> Vec<String> {
    let items: Vec<&Item> = file
        .ast
        .items
        .iter()
        .filter(|item| !matches!(item, Item::Use(_) | Item::Mod(_)))
        .collect();
    let api_traits: Vec<(usize, &syn::ItemTrait)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| match item {
            Item::Trait(t) if t.ident.to_string().ends_with("Api") => Some((i, t)),
            _ => None,
        })
        .collect();
    let [(trait_at, api_trait)] = api_traits.as_slice() else {
        return vec![format!(
            "declares {} `…Api` traits instead of one",
            api_traits.len()
        )];
    };
    let api = api_trait.ident.to_string();
    let service = api.trim_end_matches("Api").to_string();
    let Some(struct_at) = position(
        &items,
        |item| matches!(item, Item::Struct(s) if s.ident == service),
    ) else {
        return vec![format!("no `pub struct {service}` for `{api}`")];
    };
    let inherent: Vec<usize> = (0..items.len())
        .filter(|&i| matches!(items[i], Item::Impl(imp) if imp.trait_.is_none() && type_name(&imp.self_ty) == service))
        .collect();
    let api_impl = position(&items, |item| {
        matches!(item, Item::Impl(imp) if type_name(&imp.self_ty) == service
            && imp.trait_.as_ref().is_some_and(|(_, path, _)| path_idents(path).last() == Some(&api)))
    });
    let mut problems = Vec::new();
    if !is_pub_send_sync(api_trait) {
        problems.push(format!(
            "`{api}` is not declared `pub trait {api}: Send + Sync`"
        ));
    }
    match (inherent.first(), api_impl) {
        (Some(&constructor), Some(api_impl)) => {
            if fn_names(items[constructor]) != ["new"] {
                problems.push(format!("the first `impl {service}` holds more than `new`"));
            }
            if !(struct_at < constructor && constructor < *trait_at && *trait_at < api_impl) {
                problems.push(format!(
                    "blocks out of order: `struct {service}`, `impl {service}` with `new`, `trait {api}`, \
                     `impl {api} for {service}`"
                ));
            }
            if inherent.len() > 2 {
                problems.push(format!(
                    "{} `impl {service}` blocks instead of at most two",
                    inherent.len()
                ));
            }
            if let Some(&helpers) = inherent.get(1) {
                if helpers < api_impl {
                    problems.push(format!(
                        "the helpers `impl {service}` comes before `impl {api} for {service}`"
                    ));
                }
                if let Item::Impl(imp) = items[helpers]
                    && impl_fns(imp).iter().any(|(_, is_pub)| *is_pub)
                {
                    problems.push(format!("the helpers `impl {service}` has a `pub fn`"));
                }
            }
            let blocks = [
                Some(struct_at),
                Some(*trait_at),
                Some(api_impl),
                inherent.first().copied(),
                inherent.get(1).copied(),
            ];
            items
                .iter()
                .enumerate()
                .skip(struct_at + 1)
                .for_each(|(i, item)| {
                    if !blocks.contains(&Some(i)) && !matches!(item, Item::Fn(_)) {
                        problems.push(format!(
                            "`{}` comes after the struct: supporting items go above it",
                            describe(item)
                        ));
                    }
                });
        }
        (None, _) => problems.push(format!("no `impl {service}` with `new`")),
        (_, None) => problems.push(format!("no `impl {api} for {service}`")),
    }
    items.iter().for_each(|item| {
        if let Item::Fn(f) = item {
            problems.push(format!(
                "free fn `{}`: helpers go in the last `impl {service}`",
                f.sig.ident
            ));
        }
    });
    problems
}

fn is_pub_send_sync(api: &syn::ItemTrait) -> bool {
    let bounds: Vec<String> = api
        .supertraits
        .iter()
        .filter_map(|bound| match bound {
            syn::TypeParamBound::Trait(t) => path_idents(&t.path).last().cloned(),
            _ => None,
        })
        .collect();
    matches!(api.vis, Visibility::Public(_)) && bounds == ["Send", "Sync"]
}

fn position(items: &[&Item], predicate: impl Fn(&Item) -> bool) -> Option<usize> {
    items.iter().position(|item| predicate(item))
}

fn fn_names(item: &Item) -> Vec<String> {
    match item {
        Item::Impl(imp) => impl_fns(imp).into_iter().map(|(name, _)| name).collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::baseline::{assert_matches_baseline, units};
    use super::super::source::repository_sources;
    use super::*;

    const WELL_LAID_OUT: &str = "use std::sync::Arc;\n\
        pub enum CreateWidgetOutcome { Created, AlreadyThere }\n\
        pub struct WidgetCreator {\n    repository: Arc<dyn WidgetRepository>,\n}\n\
        impl WidgetCreator {\n    pub fn new(repository: Arc<dyn WidgetRepository>) -> Self {\n        Self { repository }\n    }\n}\n\
        pub trait WidgetCreatorApi: Send + Sync {\n    fn create(&self, id: WidgetId) -> Result<Widget, CreateWidgetError>;\n}\n\
        impl WidgetCreatorApi for WidgetCreator {\n    fn create(&self, id: WidgetId) -> Result<Widget, CreateWidgetError> {\n        self.ensure_absent(&id)\n    }\n}\n\
        impl WidgetCreator {\n    fn ensure_absent(&self, id: &WidgetId) -> Result<Widget, CreateWidgetError> {\n        todo!()\n    }\n}\n\
        #[cfg(test)]\nmod tests {\n    fn helper() {}\n}\n";

    #[test]
    fn it_should_hold_on_the_repository() {
        assert_matches_baseline(&RULE, violations(repository_sources()));
    }

    #[test]
    fn it_should_report_a_service_with_a_pub_helper_and_a_free_fn() {
        let source = WELL_LAID_OUT
            .replace("    fn ensure_absent", "    pub fn ensure_absent")
            .replace("#[cfg(test)]", "fn slugify() {}\n#[cfg(test)]");
        let files = [SourceFile::new(
            "src/domain/services/widget_creator.rs",
            &source,
        )];

        assert_eq!(
            violations(&files)
                .into_iter()
                .map(|v| v.detail)
                .collect::<Vec<_>>(),
            vec![
                "src/domain/services/widget_creator.rs: the helpers `impl WidgetCreator` has a `pub fn`",
                "src/domain/services/widget_creator.rs: free fn `slugify`: helpers go in the last `impl WidgetCreator`",
            ]
        );
    }

    #[test]
    fn it_should_report_a_service_without_an_api_trait() {
        let files = [SourceFile::new(
            "src/domain/services/widget_creator.rs",
            "pub struct WidgetCreator;\nimpl WidgetCreator {\n    pub fn create(&self) {}\n}\n",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/domain/services/widget_creator.rs"]
        );
    }

    #[test]
    fn it_should_accept_a_service_laid_out_in_four_blocks() {
        let files = [SourceFile::new(
            "src/domain/services/widget_creator.rs",
            WELL_LAID_OUT,
        )];

        assert_eq!(violations(&files), vec![]);
    }
}
