//! Within an `impl` block, and among a module's free fns: `new` first, then
//! the public ones (trait-impl methods count as public), then the private ones.

use syn::Item;

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, impl_fns, is_pub, path_idents, type_name};

pub const RULE: Rule = Rule {
    name: "fn_ordering",
    fix: "Order the fns of each `impl` block, and the free fns of each module: `new` (if any) first, \
          then every `pub` fn, then the private helpers (a repository's `row_to_*` helpers too).",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Fn ordering)",
};

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            let mut found = Vec::new();
            check_module(file, &file.ast.items, &[], &mut found);
            found
        })
        .collect()
}

fn check_module(file: &SourceFile, items: &[Item], modules: &[String], found: &mut Vec<Violation>) {
    let free_fns: Vec<(String, bool)> = items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(f) => Some((f.sig.ident.to_string(), is_pub(&f.vis))),
            _ => None,
        })
        .collect();
    let scope: String = modules.iter().map(|m| format!("{m}::")).collect();
    if let Some(problem) = misordering(&free_fns) {
        found.push(Violation::new(
            format!("{}::{scope}(free fns)", file.path),
            format!(
                "{}: free fns of `{}`: {problem}",
                file.path,
                if scope.is_empty() { "the file" } else { &scope }
            ),
        ));
    }
    items.iter().for_each(|item| match item {
        Item::Impl(imp) => {
            let is_trait_impl = imp.trait_.is_some();
            let fns: Vec<(String, bool)> = impl_fns(imp)
                .into_iter()
                .map(|(name, is_pub)| (name, is_pub || is_trait_impl))
                .collect();
            if let Some(problem) = misordering(&fns) {
                let ty = type_name(&imp.self_ty);
                let block = imp
                    .trait_
                    .as_ref()
                    .and_then(|(_, path, _)| path_idents(path).last().cloned())
                    .map_or(ty.clone(), |t| format!("{t} for {ty}"));
                found.push(Violation::new(
                    format!("{}::{scope}impl {block}", file.path),
                    format!(
                        "{}:{}: `impl {block}`: {problem}",
                        file.path,
                        file.line_of(&imp.impl_token)
                    ),
                ));
            }
        }
        Item::Mod(module) => {
            if let Some((_, inner)) = &module.content {
                let mut path = modules.to_vec();
                path.push(module.ident.to_string());
                check_module(file, inner, &path, found);
            }
        }
        _ => {}
    });
}

fn misordering(fns: &[(String, bool)]) -> Option<String> {
    if fns.iter().skip(1).any(|(name, _)| name == "new") {
        return Some("`new` is not the first fn".to_string());
    }
    let rest: Vec<&(String, bool)> = fns.iter().filter(|(name, _)| name != "new").collect();
    rest.iter().enumerate().find_map(|(i, (private, is_pub))| {
        (!is_pub)
            .then(|| rest[i + 1..].iter().find(|(_, later_pub)| *later_pub))
            .flatten()
            .map(|(public, _)| format!("`pub fn {public}` comes after private `fn {private}`"))
    })
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
    fn it_should_report_misordered_impl_blocks_and_free_fns() {
        let files = [SourceFile::new(
            "src/domain/widget/widget.rs",
            "impl Widget {\n    pub fn name(&self) {}\n    pub fn new() -> Self { Self }\n}\n\
             impl WidgetRepository for SqliteWidgetRepository {\n    fn find(&self) {}\n}\n\
             impl SqliteWidgetRepository {\n    fn row_to_widget() {}\n    pub(crate) fn list_all() {}\n}\n\
             fn helper() {}\npub fn entry() {}\n",
        )];

        assert_eq!(
            units(&violations(&files)),
            vec![
                "src/domain/widget/widget.rs::(free fns)",
                "src/domain/widget/widget.rs::impl SqliteWidgetRepository",
                "src/domain/widget/widget.rs::impl Widget",
            ]
        );
    }

    #[test]
    fn it_should_accept_new_then_public_then_private() {
        let files = [SourceFile::new(
            "src/domain/widget/widget.rs",
            "impl Widget {\n    pub fn new() -> Self { Self }\n    pub fn name(&self) {}\n    fn helper(&self) {}\n}\n\
             impl Display for Widget {\n    fn fmt(&self) {}\n}\n\
             pub fn entry() {}\nfn helper() {}\n\
             #[cfg(test)]\nmod tests {\n    #[test]\n    fn it_should_work() {}\n    fn helper() {}\n}\n",
        )];

        assert_eq!(violations(&files), vec![]);
    }
}
