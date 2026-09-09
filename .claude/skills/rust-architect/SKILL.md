---
name: rust-architect
description: sergigp's architecture, naming, and testing conventions for Rust projects — three-plus-one layering (domain/adapter/infrastructure, plus cli when there is one), file-per-concept naming, CQS repositories with anyhow errors, port placement and naming, and it_should_..._when_... test naming with hand-written fakes. Use when writing, reviewing, or refactoring Rust code: creating a new module or file, adding a repository/port/service/entity/value object, adding an HTTP handler or CLI command, deciding where a new .rs file goes, or writing/naming Rust tests.
---

# Rust Architect

sergigp's opinions on how Rust code should be structured, built up and stress-tested across
real refactors in the `yarrtube` project. Apply these by default in any Rust project; treat
them as defaults to state and confirm, not silently override, when a project's existing
conventions clearly disagree.

If `OPINIONS.md` exists at the repo root, it's the full history/rationale this skill was
distilled from — read it for the "why" behind a rule and for a project-specific changelog of
what's been decided. This file is the condensed, operational version for day-to-day coding.

## Quick decision guide

Given a new piece of code, ask in order:

1. **Is it a business rule, validation, or orchestration of a use case?** → `domain/`.
2. **Is it translating a wire format (HTTP JSON, CLI args) into a domain call and back?**
   → an adapter layer named after the wire format (`http/`, `cli/`).
3. **Does it implement a port (trait) the domain depends on?** → `infrastructure/repositories/`.
4. **Does it implement a port-shaped trait nothing in `domain/` injects** (a CLI command
   constructs and calls it directly)? → `infrastructure/client/`.
5. **Is it infra-level code with no trait/port boundary at all** (plain functions/clients used
   directly by whichever layer needs them)? → `infrastructure/shared/`.

Never put orchestration logic in an adapter layer (HTTP controller, CLI command) — it calls
exactly one domain method and translates the result. Never put wire-format or CLI-parsing
concerns in `domain/`.

## Layering

```
src/
  domain/
    <aggregate>/
      <value_object>.rs   # one file per value object, named after the type
      <entity>.rs         # named after the type, even if it repeats the folder name
      errors.rs           # every error type for this aggregate, together
      service.rs          # ALL orchestration logic for this aggregate's operations
  http/  (or grpc/, etc. — one per external interface)
    mod.rs                 # AppState + router wiring only
    error.rs               # shared response-mapping helpers
    <resource>/
      mod.rs                # handlers only
      dto.rs                # request/response wire types + From<Domain> conversions
  cli/                      # only if the project has a CLI
    mod.rs                  # Cli/Commands definitions (this is the CLI's "routing")
    <command>.rs             # one file per command's orchestration
  infrastructure/
    repositories/            # port implementations a domain service injects
      <port>.rs               # trait + its implementation, together
    client/                  # port-shaped adapters nothing in domain/ injects
      <port>.rs
    shared/                  # infra code with no trait/port boundary at all
      <thing>.rs
```

A single-file module that outgrows itself becomes a folder (`playlists.rs` →
`playlists/mod.rs` + `playlists/dto.rs`), never stays flat and bloats. `mod.rs`/root files
are composition roots only (state structs, routing, re-exports, submodule declarations) —
they are not where a shared helper goes once it needs its own logic; give it a file named
after the concept it represents (an `error.rs` for shared error-response helpers, not a
scoop-everything-into-`mod.rs` reflex).

## File naming

- No generic filenames: not `entity.rs`, not `value_objects.rs`, not `ports.rs`. A file is
  named after the single type/concept it holds (`playlist.rs` holds `Playlist`,
  `youtube_playlist_id.rs` holds `YoutubePlaylistId`). If this causes a clippy
  `module_inception` warning (a file named the same as its parent folder), silence it
  explicitly with `#[allow(clippy::module_inception)]` rather than renaming around it —
  the naming rule wins.
- `errors.rs` is the one deliberately generic name: every error type for an aggregate
  (validation errors, use-case error enums) lives together in one file, not scattered
  across the files that raise them.
- Every value object gets its own file. Don't bundle multiple value objects into one
  "value objects" file.

## Domain services: orchestration lives here, not in adapters

One `service.rs` per aggregate holds **all** the orchestration logic for that aggregate's
operations — validating input, deciding what an operation's outcome means, calling ports in
the right order. "How to create a widget" is domain logic, never controller/command logic.
An adapter (HTTP handler, CLI command) does three things only: parse/deserialize input, call
exactly one domain service method, map the result to its output format (status code,
printed output). No business rules, no validation, no multi-step orchestration in adapters.

A service is constructed with only the ports *some operation on that aggregate* actually
needs — never a dependency the aggregate's operations don't touch. This is enforced at the
service (aggregate) boundary, not per-method: a service legitimately holds a port that only
one of its methods uses. It's violated only when a service holds a port *no* operation on
that aggregate uses — that port belongs to a different aggregate's service.

Folding orchestration into `domain/<aggregate>/service.rs` (rather than a separate
`application/` layer, as stricter DDD would have it) is a deliberate simplification for
small-to-medium projects — state it explicitly if a project's scale later justifies splitting
use-case orchestration out from pure domain rules.

## Ports: placement, naming, and the CQS contract

**Placement:** Port traits are defined in `infrastructure/{repositories,client}/`, in the
same file as their (usually sole) implementation — not in `domain/`. This is a conscious,
acknowledged departure from textbook hexagonal architecture (domain owning the contract),
made for editing convenience: adding a method means touching the trait and its impl in one
file instead of two. The consequence: `domain/` depends on `infrastructure/` for these
trait imports, backwards from strict hexagonal — accept this, don't "fix" it by moving
traits back to `domain/` unless a project explicitly decides to prioritize dependency
direction over editing convenience.

**Naming:** A port is named after the concept it fronts, not the one method it happens to
expose today (`WidgetRepository`, not `WidgetLookup`, even if today it only has an `exists`
method) — "repository" is used loosely for "adapter implementing a domain port," not
strictly persistence. Renaming a port once callers exist is pure churn; name it right the
first time.

**CQS contract, no typed infra errors:** A repository/port method either **reads** (returns
`anyhow::Result<Entity>` / `anyhow::Result<Vec<Entity>>` / `anyhow::Result<Option<Entity>>`
— an entity or collection, never a wrapper/outcome enum) or **writes** (returns
`anyhow::Result<()>` — void on success). No custom infra error types
(`RepositoryError`/`LookupError` and friends) — infra failures are untyped `anyhow::Error`.
A boolean existence/predicate check is a reasonable exception to "reads return entities" —
there's often no meaningful entity to return for e.g. an external existence check.

Business-meaningful outcomes that might seem like they belong in the repository (e.g. "was
this newly created, or did it already exist?") are **computed in the domain service**, not
returned by infra: call the read method, decide, then call the write method — e.g.
`create_widget` calls `repository.find(id)`; `Some` is the idempotent-existing case, `None`
means call `repository.insert(...)` and construct the "created" outcome in the service. The
service is where untyped `anyhow::Error` becomes something meaningful — its own error enum,
with variants distinguished by *which port call* produced the error
(`Lookup(anyhow::Error)` / `Repository(anyhow::Error)`), not by inspecting the error's type.
When a method has only one possible failure source and nothing domain-specific to add, the
service may return `anyhow::Result<T>` directly rather than inventing a single-variant enum.

**Known cost — flag it, don't silently eat it:** splitting an atomic "insert-or-return-existing"
DB operation into domain-orchestrated `find` (read) then `insert` (write) trades away
atomicity — a race window opens between the two calls. If a project's existing behavior
relies on DB-level atomicity (upserts, "claim the first one" patterns, counters), don't apply
the CQS split to it without naming this trade-off out loud and getting an explicit decision.

**State transitions live on the entity, never as behavior-named repository methods.** A
repository's write side is CRUD only — `insert`/`schedule`/`create`, `update`, `delete`, plus
one atomic multi-statement write when a use case genuinely needs it (see above). It never
grows a method named after a business transition (`mark_running`, `mark_done`,
`mark_failed_or_retry`, `recover_running` and friends) that itself decides the transition —
computing new state, thresholds, or backoff timing inside the repository. That decision
belongs on a persisted domain entity as a method consuming `self` (and `now: DateTime<Utc>`
where timestamps matter) and returning either the next state or an outcome enum when a
transition can branch into different shapes (e.g. `fail(error, now) -> Retry(Entity) |
DeadLetter(DeadLetteredEntity)`, mirroring the CQS "reads return entities" contract instead of
a bespoke result type). The caller — a domain service, or a poller like a task executor/event
consumer — reads the current entity (`find`/`list_*`), calls the transition method, then
persists whatever came back via the plain CRUD write. This is the same "decide in the domain,
persist via CRUD" split as the idempotent-create pattern above, applied to lifecycle state
instead of existence checks. If you see a repository method whose name is a verb describing a
business outcome rather than a storage operation, that logic has leaked out of the domain —
move it, don't add another one next to it.

## infrastructure/ subfolders

Split by *why* the code lives in `infrastructure/`, not by an unqualified "adapters" catch-all:
- **`repositories/`** — implements a port a domain service injects.
- **`client/`** — implements a port-shaped trait (still trait + implementation, for
  swappability/testability), but nothing in `domain/` injects it — typically a CLI command
  constructs and calls it directly.
- **`shared/`** — infra-level code with no trait/port boundary at all: plain functions or
  clients used directly by whichever layer needs them. Add this folder only once something
  actually doesn't belong to a single port implementation — don't create it empty up front.
  The `shared/` vs `client/` line is whether a trait exists, not how "important" the code
  feels; promoting a `shared/` file to a trait-backed `client/`/`repositories/` entry once a
  swap/fake need shows up is expected, not a sign the original placement was wrong.

## Testing

**Behavior-driven, mock only at the boundary, public API only.** Tests verify observable
behavior (return values, status codes, persisted state) through the public API — never
private fields or visibility tricks. The only things replaced with a test double are the
**ports at the edge of the layer under test** (a domain service's tests fake its injected
traits) — nothing *inside* that layer gets mocked out. One behavior per test; a failing test
name alone should say what broke.

**Hand-written fakes, not interaction-mocking libraries.** A test double is a small, real (if
simplified) implementation of the trait — state-based, e.g. a fake repository backed by a
`Mutex<Vec<Entity>>` — not a call-expectation mock (no `mockall`/`#[automock]`/
`.expect_x().returning(...)`, no new mocking-library dependency). Fakes are simpler for
traits this small and exercise realistic behavior (insert-then-find, idempotent create)
instead of asserting on call counts/arguments.

**Infrastructure tests use the real dependency, colocated with the code.** "Infrastructure
test" (a port implementation against something real) vs. "behavior test" (domain logic
against fakes) is a real, useful distinction — but *what counts as real* depends on the
dependency:
- Embedded engine (SQLite, etc.) → an in-memory instance, not a container — in-memory *is*
  the real engine.
- Networked service (Postgres, Kafka, Redis, ClickHouse...) → testcontainers, when a project
  actually has one. Don't reach for testcontainers or a mocking framework for a dependency a
  project doesn't have.
- Outbound HTTP dependency → the real HTTP client against a mock server (`mockito`), not a
  container.

Keep tests in `#[cfg(test)] mod tests` in the same file as the code they test by default. A
top-level `tests/` directory needs a `lib.rs` (the project becomes a library + thin binary)
purely to give integration tests access to internal types — don't introduce that structural
change just to match a template; do it when a project independently justifies a `lib.rs`
(e.g. multiple binaries need to share more than modules already give them).

**Test naming:** `it_should_<expected outcome>_when_<condition>` — drop the `_when_...` part
when there's no meaningful precondition beyond "given valid input"
(`it_should_build_the_widget_url`). When adopting this in a project with existing tests,
rename the existing ones too — don't let two conventions coexist.

**Determinism and test data:** time is deterministic via the same `Clock` port used in
production (a `FixedClock`) — this should require no special-casing, it falls out of already
having `Clock` as an injected port. Build test data through small helper functions with
sensible defaults (`fn widget(id, name) -> Widget`), parameterized only on what varies per
test. Domain-specific assertion helpers and nested `mod <method_name> { ... }` grouping
inside `mod tests` are patterns to *graduate to* once a test file's size/repetition earns
them — not a starting template for a handful of tests.

## Deliberate deviations — don't "fix" these

A few rules above are explicit, discussed trade-offs, not oversights. If code in a project
following this skill looks like it violates textbook architecture in one of these specific
ways, it's probably intentional:
- Port traits live in `infrastructure/`, not `domain/` (dependency direction inverted from
  strict hexagonal, traded for editing convenience).
- Repositories have no typed error enums — everything is `anyhow::Error` until the domain
  service gives it meaning.
- An idempotent "create" use case is two non-atomic port calls (`find` then `insert`), not
  one atomic upsert — a narrow, named race-condition trade-off, not an oversight.
- Orchestration lives in `domain/<aggregate>/service.rs`, not a separate `application/`
  layer — a deliberate simplification for project scale, not missing layering.

When something looks off, say so and ask before changing it — these were each surfaced and
confirmed explicitly once; they shouldn't need to be re-litigated silently.
