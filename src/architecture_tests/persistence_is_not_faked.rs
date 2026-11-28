//! Tests wire the real `Sqlite*` repositories: no test double of a port that
//! SQLite implements. A wrapper holding the real `Sqlite*` implementation of
//! the port (to inject a failure or a race around it) is not a fake.

use std::collections::{BTreeMap, BTreeSet};

use syn::Item;

use super::baseline::{Rule, Violation};
use super::source::{SourceFile, path_idents, type_name};

pub const RULE: Rule = Rule {
    name: "persistence_is_not_faked",
    fix: "Use the real `Sqlite*` repository on `let db = TestDatabase::new();` \
          (`SqliteXRepository::new(db.connection())`), seeding and asserting through the port. Fakes \
          are only for external or side-effecting dependencies (YouTube, yt-dlp, HTTP, filesystem); \
          to inject a failure or a race, wrap the real `Sqlite*` in a field and delegate to it.",
    convention: "Convention: .claude/skills/rust-architect/SKILL.md (Persistence and Test Doubles)",
};

pub fn violations(files: &[SourceFile]) -> Vec<Violation> {
    let crate_traits: BTreeSet<String> = files
        .iter()
        .flat_map(|file| file.all_items())
        .filter_map(|scoped| match scoped.item {
            Item::Trait(t) => Some(t.ident.to_string()),
            _ => None,
        })
        .collect();
    // Each port SQLite implements → its `Sqlite*` implementations.
    let sqlite_ports: BTreeMap<String, BTreeSet<String>> = files
        .iter()
        .flat_map(|file| file.all_items())
        .filter(|scoped| !scoped.in_test)
        .filter_map(|scoped| trait_impl(scoped.item))
        .filter(|(port, ty)| ty.starts_with("Sqlite") && crate_traits.contains(port))
        .fold(BTreeMap::new(), |mut ports, (port, ty)| {
            ports.entry(port).or_default().insert(ty);
            ports
        });
    let test_struct_fields: BTreeMap<String, BTreeSet<String>> = files
        .iter()
        .flat_map(|file| file.all_items())
        .filter(|scoped| scoped.in_test)
        .filter_map(|scoped| match scoped.item {
            Item::Struct(s) => Some((
                s.ident.to_string(),
                s.fields.iter().map(|field| type_name(&field.ty)).collect(),
            )),
            _ => None,
        })
        .collect();
    let mut doubles: BTreeMap<String, Violation> = BTreeMap::new();
    files.iter().for_each(|file| {
        file.all_items()
            .into_iter()
            .filter(|scoped| scoped.in_test)
            .for_each(|scoped| {
                if let Some((port, ty)) = trait_impl(scoped.item)
                    && let Some(implementations) = sqlite_ports.get(&port)
                    && !ty.starts_with("Sqlite")
                    && !wraps_one_of(test_struct_fields.get(&ty), implementations)
                {
                    let unit = format!("{}::{ty}", file.path);
                    let detail = format!(
                        "{}:{}: test double `{ty}` of `{port}`, which SQLite implements",
                        file.path,
                        file.line_of(scoped.item)
                    );
                    doubles.insert(unit.clone(), Violation::new(unit, detail));
                }
            })
    });
    doubles.into_values().collect()
}

/// Whether a struct's fields hold one of the port's `Sqlite*` implementations.
fn wraps_one_of(fields: Option<&BTreeSet<String>>, implementations: &BTreeSet<String>) -> bool {
    fields.is_some_and(|fields| !fields.is_disjoint(implementations))
}

/// `impl Port for Type` → (Port, Type).
fn trait_impl(item: &Item) -> Option<(String, String)> {
    match item {
        Item::Impl(imp) => imp
            .trait_
            .as_ref()
            .and_then(|(_, path, _)| path_idents(path).last().cloned())
            .map(|port| (port, type_name(&imp.self_ty))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::baseline::{assert_matches_baseline, units};
    use super::super::source::repository_sources;
    use super::*;

    fn sqlite_port() -> SourceFile {
        SourceFile::new(
            "src/infrastructure/repositories/sqlite_widget_repository.rs",
            "pub trait WidgetRepository: Send + Sync {}\npub struct SqliteWidgetRepository;\n\
             impl WidgetRepository for SqliteWidgetRepository {}\n",
        )
    }

    #[test]
    fn it_should_hold_on_the_repository() {
        assert_matches_baseline(&RULE, violations(repository_sources()));
    }

    #[test]
    fn it_should_report_a_fake_of_a_sqlite_port() {
        let files = [
            sqlite_port(),
            SourceFile::new(
                "src/application/http/widgets/tests.rs",
                "struct FakeWidgetRepository;\nimpl WidgetRepository for FakeWidgetRepository {}\n",
            ),
        ];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/application/http/widgets/tests.rs::FakeWidgetRepository"]
        );
    }

    #[test]
    fn it_should_accept_a_wrapper_of_the_sqlite_implementation() {
        let files = [
            sqlite_port(),
            SourceFile::new(
                "src/application/http/widgets/tests.rs",
                "struct FailingWidgetRepository(SqliteWidgetRepository);\n\
                 impl WidgetRepository for FailingWidgetRepository {}\n\
                 struct FakeWidgetRepository(Vec<Widget>);\n\
                 impl WidgetRepository for FakeWidgetRepository {}\n",
            ),
        ];

        assert_eq!(
            units(&violations(&files)),
            vec!["src/application/http/widgets/tests.rs::FakeWidgetRepository"]
        );
    }

    #[test]
    fn it_should_accept_fakes_of_external_ports() {
        let files = [
            sqlite_port(),
            SourceFile::new(
                "src/infrastructure/repositories/youtube_thing_repository.rs",
                "pub trait ThingRepository {}\n#[cfg(test)]\nmod tests {\n    struct FakeThingRepository;\n    \
                 impl ThingRepository for FakeThingRepository {}\n    impl Default for FakeThingRepository {}\n}\n",
            ),
        ];

        assert_eq!(violations(&files), vec![]);
    }
}
