//! Within an `impl` block, and among a module's free fns: `new` first, then
//! the public ones (trait-impl methods count as public), then the private ones.

use super::baseline::{Rule, Violation};
use super::source::{Item, ItemKind, SourceFile};

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
            check_module(file, &file.items(), &[], &mut found);
            found
        })
        .collect()
}

fn check_module(file: &SourceFile, items: &[Item], modules: &[String], found: &mut Vec<Violation>) {
    let free_fns: Vec<(String, bool)> = items.iter().filter_map(fn_entry).collect();
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
    items.iter().for_each(|item| match &item.kind {
        ItemKind::Impl { ty, trait_ } => {
            let fns: Vec<(String, bool)> = file
                .children(item)
                .iter()
                .filter_map(fn_entry)
                .map(|(name, is_pub)| (name, is_pub || trait_.is_some()))
                .collect();
            if let Some(problem) = misordering(&fns) {
                let block = trait_
                    .as_ref()
                    .map_or(ty.clone(), |t| format!("{t} for {ty}"));
                found.push(Violation::new(
                    format!("{}::{scope}impl {block}", file.path),
                    format!(
                        "{}:{}: `impl {block}`: {problem}",
                        file.path,
                        file.line_of(item.start)
                    ),
                ));
            }
        }
        ItemKind::Mod { name } => {
            let mut inner = modules.to_vec();
            inner.push(name.clone());
            check_module(file, &file.children(item), &inner, found);
        }
        _ => {}
    });
}

fn fn_entry(item: &Item) -> Option<(String, bool)> {
    match &item.kind {
        ItemKind::Fn { name, is_pub } => Some((name.clone(), *is_pub)),
        _ => None,
    }
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
