# Rust Architecture Opinions

Working doc capturing sergigp's opinions on how Rust code in this codebase (and future ones)
should be structured and written. Built incrementally in conversation with Claude — each
opinion is challenged before being accepted, then generalized so it holds beyond this one
repo. This file is the source material for a future `rust-architect` skill.

Status: **draft, in progress.**

## How entries are written

Each opinion gets:
- **Rule** — the general, project-agnostic statement.
- **Why** — the reasoning (including any pushback that was resolved).
- **Applies when / exceptions** — where it's a hard rule vs. a default that can bend.

---

## Opinions

### 1. Prefer many small, single-purpose files over few large ones

**Rule:** Split a module by *responsibility*, not just by "this file got long." In particular,
for HTTP/API layers, separate at minimum:
- **handlers** — the actual request/response orchestration logic
- **dto** — request/response wire types (`#[derive(Deserialize)]` / `#[derive(Serialize)]` structs
  and their `From<Domain>` conversions), one file per resource
- **shared cross-cutting helpers** (e.g. error formatting) — their own file, not bolted onto
  whatever `mod.rs`/root file happens to already exist

A resource that outgrows a single file becomes a folder (`playlists.rs` → `playlists/mod.rs` +
`playlists/dto.rs`) rather than staying flat and bloating.

**Why:** Cluttered files mix concerns (wire format vs. business orchestration vs. cross-cutting
infra) and make it harder to find/change one thing without scanning the rest. The rule was
refined during discussion: the initial suggestion was to dump shared helpers like
`error_response` into `mod.rs`, but that just relocates the clutter — `mod.rs`/root files are
composition roots (module wiring, shared state) and become the new dumping ground if every
handler file pushes its helpers there as the app grows. Shared cross-cutting concerns get their
own file instead.

**Applies when / exceptions:** This is a default, not dogma — a resource with one trivial DTO
and no shared helpers doesn't need a folder yet. Split when a file is doing more than one kind
of job, not on line count alone. `mod.rs`/root files should stay limited to composition
(re-exports, state structs, wiring) — if a helper function or type belongs to a concept (errors,
auth, pagination), it gets a file named after that concept.

### 2. Three top-level layers: domain, http, infrastructure — orchestration lives in domain

**Rule:** The codebase has exactly three top-level layers:
- **`domain/`** — one subfolder per aggregate/domain concept (e.g. `domain/playlist/`).
  Inside each: one file per value object, one file per entity (named after the concept
  itself, not `value_objects.rs`/`entity.rs` — see opinion #3), an `errors.rs`, and a
  `service.rs` — one service per aggregate that holds **all the orchestration logic for
  that aggregate's operations** ("how to create a playlist" is domain logic, not
  controller logic). A service is constructed with only the ports *some operation on
  that aggregate* actually needs — never a dependency the aggregate's operations don't
  touch.
- **`http/`** — adapter only. Controllers extract/deserialize the request, call exactly
  one domain service method, and map the `Result` to a status code + response body.
  No business rules, no validation logic, no orchestration — just translation between
  wire format and domain calls. Request/response DTOs live next to their controller
  (`http/<resource>/dto.rs`), per opinion #1.
- **`infrastructure/`** — adapters that *implement* domain ports (as of opinion #3, the
  port trait itself is also defined here, next to its implementation — a deliberate,
  acknowledged deviation from strict hexagonal ownership; see #3 for why).
  `repositories/` holds every port implementation regardless of whether it's backed by
  persistence (SQL) or an external system (a YouTube API client, a system clock) —
  "repository" is used loosely here for "adapter implementing a domain port," not
  strictly persistence. A `shared/` folder is added later, only once something
  infra-level doesn't belong to a single port implementation (e.g. a shared DB
  connection pool) — don't create it empty up front.

**Why:** Keeps business rules (what does "create a playlist" mean, what makes it valid,
what does idempotent creation return) in one place, testable without HTTP/DB, and keeps
the HTTP layer swappable (could add a CLI or gRPC adapter calling the same service
without duplicating logic). Refined during discussion: the initial framing was strict
DDD (separate `application` layer for use-case orchestration, `domain` reserved for pure
entities/rules), but for this project's size the user chose to fold orchestration into
`domain/<aggregate>/service.rs` directly rather than add a fourth layer — a deliberate
simplification, not an oversight.

**Applies when / exceptions:** A service is scoped per aggregate, not per single
operation — `PlaylistService` legitimately holds every port *any* playlist operation
needs (repository, YouTube lookup, clock), even though `delete_playlist` alone only
touches the repository. The "only inject what's needed" rule operates at the
aggregate/service boundary, not enforced per method. It's still violated if a service
gets a port that *no* operation on that aggregate uses — that's a sign the port belongs
to a different aggregate's service instead.

*Superseded:* an earlier version of this opinion put port trait definitions in
`domain/<aggregate>/ports.rs` (domain owns the contract, infra only implements it) —
this was **overturned in opinion #3**, which is now the source of truth for where port
traits live.

### 3. Ports live with their implementation; repositories are pure CQS; one type per file

**Rule, part A — file naming:** No generic filenames (`entity.rs`, `value_objects.rs`,
`ports.rs`, `errors.ts`/`.rs` is the one exception — see part D). A file is named after
the single type/concept it holds: `playlist.rs` holds `Playlist` (even though it sits in
a `domain/playlist/` folder — the module-inception clippy lint this causes is silenced
explicitly, not avoided by renaming), `youtube_playlist_id.rs` holds `YoutubePlaylistId`,
`playlist_name.rs` holds `PlaylistName`. Every value object gets its own file.

**Rule, part B — ports live next to their implementation:** Port traits (`PlaylistRepository`,
`YoutubePlaylistLookup`, `Clock`) are defined in `infrastructure/repositories/`, in the
same file as their (usually sole) implementation — not in `domain/`. This is a conscious
departure from textbook hexagonal architecture (where the domain owns the contract) made
for editing convenience: adding a method means changing the trait and its impl in one
file instead of two. The domain service (`domain/<aggregate>/service.rs`) imports the
trait from `infrastructure::repositories::...` — meaning `domain/` now depends on
`infrastructure/`, backwards from strict hexagonal architecture, and the aggregate's unit
tests must depend on infra module paths for their fakes' trait imports.

**Rule, part C — repositories are pure CQS, no typed errors:** A repository/port method
either **reads** (returns `anyhow::Result<Entity>` or `anyhow::Result<Vec<Entity>>` — an
entity or collection of entities, never a wrapper/outcome enum) or **writes** (returns
`anyhow::Result<()>` — void on success, nothing else). No custom error types
(`RepositoryError`, `LookupError`) — infra failures are untyped `anyhow::Error`. A
boolean existence/predicate check (e.g. `YoutubePlaylistLookup::exists`) is treated as a
reasonable exception to "reads return entities" — there's no meaningful entity to return
for an external existence check.

Business-meaningful outcomes that used to come from the repository (e.g. "was this
newly created or did it already exist?") are now **computed in the domain service**, not
returned by infra: the service calls the read method first, decides, then calls the
write method. E.g. `create_playlist` calls `repository.find(id)` — if `Some`, it's the
idempotent-existing case; if `None`, it calls `repository.insert(...)` and constructs the
"created" outcome itself. The domain service is where infra's untyped `anyhow::Error` is
turned into something meaningful — its own error enum with variants like
`CreatePlaylistError::Lookup(anyhow::Error)` / `Repository(anyhow::Error)`, distinguished
by *which port call* produced the error, not by inspecting the error's type. When a
method has only one possible failure source and there's nothing domain-specific to add
(e.g. `list_playlists`), the service may return `anyhow::Result<T>` directly rather than
inventing a single-variant wrapper enum.

**Rule, part D:** an aggregate's error types (validation errors, use-case error enums)
live together in one `errors.rs` per aggregate, rather than scattered across the files
that raise them.

**Why:** Parts A and D are opinion #1 (small, single-purpose files) applied consistently
to the domain layer. Part B is an explicit, acknowledged trade-off: convenience of
co-located trait+impl over strict dependency-direction purity — accepted with eyes open,
not a mistake to "fix" later. Part C turns repositories into simple, predictable CQS
primitives and pushes all business interpretation of "what happened" into the domain,
which is where opinion #2 already says orchestration belongs.

**Applies when / exceptions — flag this one, don't silently replicate it:** splitting an
atomic "insert-or-return-existing" DB operation into a domain-orchestrated
`find` (read) then `insert` (write) trades away atomicity. The previous implementation
used the database's own primary-key constraint to make duplicate-create idempotent in a
single round trip; the CQS version has a race window between `find` and `insert` — two
truly concurrent creates of the same brand-new ID can both see "not found" and both
attempt `insert`, and the DB's primary-key constraint will still reject the second one,
which now surfaces as a generic `CreatePlaylistError::Repository` (mapped to a 500)
instead of gracefully resolving to the idempotent "already existed" response. Accepted
here as a narrow, low-probability edge case for this app's concurrency profile — but
this is a real correctness cost of the CQS-repository rule for any operation whose
current behavior depends on DB-level atomicity (upserts, "claim the first one" patterns,
counters). Don't apply part C to such an operation without naming this trade-off and
getting an explicit decision, the way it was surfaced and accepted here.

### 4. Naming and placement: "repository" over "lookup"/"client" for domain ports; a `cli/` layer; infra `client/` vs `shared/`

**Rule, part A — port names say "repository," not the verb of the one operation they
currently expose:** A port that reads/checks state through an external system (e.g. "does
this YouTube playlist exist?") is still named `<Thing>Repository`
(`YoutubePlaylistRepository`, not `YoutubePlaylistLookup`), consistent with opinion #2/#3's
loose use of "repository" for "adapter implementing a domain port." Don't let a port's name
describe today's single method (`exists`) — name it after the concept it fronts.

**Rule, part B — a CLI gets its own top-level layer, structured like the others:** Command
entry points and their per-command logic live under `cli/`, mirroring the `http/` shape from
opinion #1/#2: `cli/mod.rs` holds the actual `Cli`/`Commands` definitions (the argument
parser is the CLI's "routing"), and each command's implementation gets its own file
(`cli/download_command.rs`, `cli/ytdlp_update.rs`) declared as a submodule from `mod.rs`. A
CLI command file is an adapter, same rules as an HTTP controller: parse input, call into
domain/infrastructure, print output — no business logic that isn't just orchestration of a
CLI run.

**Rule, part C — infrastructure gets more than one subfolder, split by role:** Beyond
`repositories/` (adapters implementing a domain port), infrastructure has:
- **`client/`** — outbound adapters that are also structured as an explicit port (trait +
  implementation, e.g. `YoutubeDownloaderClient` / `YtDlpDownloaderClient`) but that a
  *domain service* doesn't inject — here, a CLI command constructs and calls it directly.
  The trait still exists (for the same reason any port does: swappable implementation,
  testability via a fake), it's just not wired through `domain/`.
- **`shared/`** — infrastructure-level code with no trait/port boundary at all: plain
  functions or clients used directly by whichever layer needs them (`youtube_api.rs`'s
  paginated fetch functions, called straight from `cli/download_command.rs`). This is the
  folder opinion #2 said to add "only once something doesn't belong to a single port
  implementation" — it's now in use.

**Why:** Part A keeps port naming predictable as a codebase grows past its first method —
renaming later, once callers exist, is pure churn. Part B applies the layering opinion (#1,
#2) to a layer that isn't HTTP — the CLI is just another adapter shape, not a special case.
Part C distinguishes three different reasons code lives in `infrastructure/`: implementing a
port a domain service depends on (`repositories/`), implementing a port nothing in `domain/`
depends on (`client/`), and having no port at all (`shared/`) — collapsing these into one
folder loses the information of *why* something is structured the way it is.

**Applies when / exceptions:** Part C's `client/` vs `shared/` distinction hinges on whether
a trait exists, not on how "important" or reusable the code feels — `youtube_api.rs` could
have been given a trait too, but wasn't asked for, so it's `shared/` until someone actually
needs to swap or fake it. If that need shows up, promoting a `shared/` file to a
trait-backed `client/` (or `repositories/`) entry is expected, not a sign the original
placement was wrong.

### 5. Testing: behavior tests with hand-written fakes, colocated with the code, `it_should_..._when_...` names

**Source:** adapted from a `rust-test-architect` doc the user brought from another (larger,
multi-service: ClickHouse/Postgres/Kafka/Redis via testcontainers) project. Most of its core
principles transferred directly; the parts that assumed a bigger, networked-services stack
were deliberately *not* copied — see "Applies when / exceptions."

**Rule, part A — behavior-driven, mock only at the boundary, public API only:** Tests
verify observable behavior (return values, status codes, persisted state) through the
type's public API — never private fields, `pub(crate)` items, or other visibility tricks.
The only things that get replaced with a test double are the **ports at the edge of the
layer under test** — a domain service's tests fake its injected traits
(`PlaylistRepository`, `YoutubePlaylistRepository`, `Clock`); nothing *inside* the domain
layer (e.g. value-object validation) gets mocked out from a service test. One behavior per
test — if it fails, what's broken should be obvious from the test name alone.

**Rule, part B — test doubles are hand-written fakes, not interaction-mocking libraries:**
A test double is a small, real (if simplified) implementation of the trait — state-based,
like `FakePlaylistRepository` backed by a `Mutex<Vec<Playlist>>` — not a call-expectation
mock (`mockall`'s `#[automock]` + `.expect_x().returning(...)`). No `mockall` dependency.
Fakes are simpler to read for traits this small (3-4 methods) and encourage testing through
realistic behavior (insert-then-find, idempotent create) rather than asserting on call
counts/arguments.

**Rule, part C — infrastructure tests use the real dependency, colocated with the code:**
"Infrastructure test" (exercises a port implementation against something real, not a fake)
vs. "behavior test" (exercises domain logic against fakes) is a real, useful distinction —
kept from the source doc. What changed is *what* counts as "real" and *where* the test
lives:
- For an embedded engine (SQLite), "real" is an in-memory instance
  (`Connection::open_in_memory()`) — not a testcontainer, because there's no separate
  service to containerize; in-memory *is* the real engine.
  For a networked service this project doesn't have yet (Postgres, ClickHouse, Kafka,
  Redis...), the source doc's testcontainers approach is still the right call if/when one
  is added.
- For an outbound HTTP dependency (the YouTube Data API, the GitHub releases API), "real"
  means exercising the actual HTTP client against a mock server (`mockito`), not a
  container.
- All of these tests stay in `#[cfg(test)] mod tests` in the same file as the code they
  test — no `tests/` directory, no `lib.rs`. Splitting to a top-level `tests/` directory
  would require introducing a `lib.rs` (yarrtube becomes a library + thin binary) purely to
  give integration tests access to internal types — a real structural change not justified
  by this project's size. Revisit if/when a `lib.rs` becomes independently justified (e.g.
  the CLI and HTTP server need to share more than modules already give them).

**Rule, part D — test naming: `it_should_<outcome>_when_<condition>`:** Every test function
is named `it_should_<expected behavior>` (drop `_when_<condition>` when there's no
meaningful precondition beyond "given valid input" — e.g. `it_should_build_the_youtube_playlist_url`).
Applies retroactively — when this rule is adopted, existing tests get renamed too, not
just new ones, so the codebase doesn't carry two conventions side by side.

**Rule, part E — determinism and test data, kept from the source doc:** Time is
deterministic via the same `Clock` port used in production (`FixedClock`) — no
special-casing for tests, it falls out of already having `Clock` as an injected port
(opinion #2/#3). Test data is built through small helper functions with sensible defaults
(`fn playlist(id, name) -> Playlist`, `fn service(youtube_exists: bool) -> PlaylistService`),
parameterized only on what varies per test. Domain-specific assertion helpers and nested
`mod <method_name> { ... }` grouping inside `mod tests` are good patterns to *graduate to*
once a test file's size/repetition earns them — not a starting template to apply on day
one for a handful of tests.

**Why:** Behavior-vs-infrastructure and mock-at-the-boundary are sound regardless of stack
size, and this codebase already followed them before this opinion made them explicit
(fakes for `PlaylistRepository`, `mockito` for the YouTube/GitHub HTTP boundary,
in-memory SQLite for the repository tests). The source doc's `mockall`/testcontainers/
`tests/`-directory choices are specific to a larger multi-service codebase with real
networked infra to spin up — applying them here would mean a new dependency (`mockall`),
a new crate shape (`lib.rs`), and container orchestration for a project with no networked
service to test against. Adopting the *principle* (real dependency, mocked boundary) while
picking the *right-sized implementation* for this project is the generalization opinion #1
already establishes for file structure, now applied to testing.

**Applies when / exceptions:** If yarrtube gains a networked external dependency
(Postgres, a message queue, etc.), testcontainers for that specific dependency is
consistent with this opinion, not a deviation from it — "use the real thing" was never
about avoiding containers specifically, it was about avoiding mocks that drift from real
behavior. Likewise, if the CLI and HTTP server start needing to share substantially more
than `domain`/`infrastructure` already give them, or the test suite grows large enough that
compiling `#[cfg(test)]` code into the binary becomes a real cost, revisit the `lib.rs` +
`tests/` question — it was deferred as unjustified *for now*, not ruled out permanently.
