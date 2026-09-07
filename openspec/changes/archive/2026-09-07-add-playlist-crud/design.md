## Context

Yarrtube today is flat and procedural (see `proposal.md` - Impact): no domain layer, no repository abstraction, no traits. `src/serve.rs` opens a `rusqlite::Connection` only to prove the DB is reachable; `src/youtube_api.rs` calls the `playlistItems` resource directly with a blocking `reqwest` client. This change introduces the project's first domain/persistence layer, following a hexagonal (ports and adapters) split with DDD-style entities, so it's worth fixing conventions here that later work (video tracking, the scheduler) will follow.

## Goals / Non-Goals

**Goals:**
- Establish a controller -> domain -> infrastructure layering, with validation/behavior on entities and value objects rather than in standalone service functions.
- Keep the number of ports minimal: only what this change's Create/Delete/List flows actually need.
- Make `created_at` deterministic in tests via a `Clock` port, rather than tests asserting against real wall-clock time.
- Reuse the same playlist-identity concept (`YoutubePlaylistId`) between the new HTTP API and the refactored CLI.

**Non-Goals:**
- Video tracking/downloading tied to a stored playlist (that's the scheduler's future work).
- A `GET /playlists/{id}` endpoint (explicitly deferred; only List was requested).
- Fetching or storing the playlist's YouTube-side title/name - the caller-supplied `name` is the only name that exists in this domain.
- A generic migrations framework - one `CREATE TABLE IF NOT EXISTS` is enough for a single table.

## Decisions

### Layering and module layout
```
src/
  domain/
    playlist.rs        # Playlist entity, YoutubePlaylistId, PlaylistName, PlaylistError
    ports.rs            # PlaylistRepository, YoutubePlaylistLookup, Clock traits
  http/
    playlists.rs        # axum handlers + request/response DTOs for /playlists
  infra/
    sqlite_playlist_repository.rs   # implements PlaylistRepository via rusqlite
    youtube_playlist_lookup.rs      # implements YoutubePlaylistLookup via YouTube `playlists` resource
    system_clock.rs                 # implements Clock via SystemTime/chrono-less UTC now
```
`serve.rs` becomes the composition root: it constructs the concrete adapters and puts them behind `Arc<dyn Trait>` (or a generic `AppState<R, L, C>`; see below) in axum's `State`.

Alternative considered: a fourth "application service" layer coordinating repository + lookup + clock. Rejected per the user's explicit ask to keep behavior on entities and only three layers - the coordination in Create is a few sequential calls that the HTTP handler itself can make directly against the ports; there's no branching logic complex enough to warrant its own layer.

### Ports as trait objects, not generics
`PlaylistRepository`, `YoutubePlaylistLookup`, and `Clock` are `dyn`-compatible traits (no generic methods), stored as `Arc<dyn Trait + Send + Sync>` in axum state. This keeps handler signatures simple and lets integration tests swap in fakes without generic parameters propagating through the router type. The small dynamic-dispatch cost is irrelevant here (a handful of calls per HTTP request).

### `Clock` port
```
trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>; // or equivalent UTC timestamp type
}
```
`SystemClock` implements it via `SystemTime::now()` (or `chrono::Utc::now()` if `chrono` is added - see Open Questions). Tests use a `FixedClock` returning a constant, so `created_at` assertions are exact-value, not range/tolerance checks.

### `YoutubePlaylistLookup` is existence-only
```
trait YoutubePlaylistLookup: Send + Sync {
    fn exists(&self, id: &YoutubePlaylistId) -> Result<bool, LookupError>;
}
```
Implemented by calling `GET /youtube/v3/playlists?id={id}&part=id&key={api_key}` and checking whether `items` is non-empty - the cheapest call that confirms existence without fetching content the domain doesn't use. This reuses the existing `YOUTUBE_API_KEY` env var and blocking `reqwest` client pattern already in `youtube_api.rs`.

### `PlaylistRepository` and idempotent create
```
enum SaveOutcome { Created(Playlist), AlreadyExisted(Playlist) }

trait PlaylistRepository: Send + Sync {
    fn save(&self, playlist: &Playlist) -> Result<SaveOutcome, RepositoryError>;
    fn delete(&self, id: &YoutubePlaylistId) -> Result<bool, RepositoryError>; // false = not found
    fn list(&self) -> Result<Vec<Playlist>, RepositoryError>;
}
```
`id` is the SQLite table's primary key (`TEXT PRIMARY KEY`). `save` relies on the primary-key constraint to detect duplicates: `SqlitePlaylistRepository` attempts the insert, and on a primary-key constraint violation, re-selects the existing row and returns `SaveOutcome::AlreadyExisted` with it instead of erroring - Create is idempotent per spec (a duplicate ID makes no change and returns the pre-existing record). This keeps "is it a duplicate" a single source of truth (the DB) while giving the HTTP handler what it needs (200 vs 201) without an extra port method. `created_at` is stored as an ISO 8601 UTC string (`TEXT`) for readability when inspecting the file directly - acceptable at this table's scale.

### `Playlist` entity and value objects
```
struct YoutubePlaylistId(String);       // non-empty; to_url() -> https://www.youtube.com/playlist?list=<id>
struct PlaylistName(String);            // non-empty, no '/' '\' or other filesystem-unsafe chars
struct Playlist { id: YoutubePlaylistId, name: PlaylistName, created_at: DateTime<Utc> }

impl Playlist {
    fn create(id: YoutubePlaylistId, name: PlaylistName, created_at: DateTime<Utc>) -> Playlist;
    // construction of the value objects themselves is where validation happens
    // (YoutubePlaylistId::try_from / PlaylistName::try_from), matching "put
    // behavior in entities, not services"
}
```
The HTTP handler is responsible for calling `YoutubePlaylistLookup::exists` before constructing the entity - existence against an external system isn't something a value object can check itself, so it stays in the handler's orchestration rather than inside `Playlist::create`.

### CLI refactor scope
`extract_playlist_id` (currently in `youtube_api.rs`, parses a `list=` query param from a URL) is removed. `download_command.rs` takes a `YoutubePlaylistId` argument directly (via `clap`'s value parsing, reusing the same value object's validation) and calls `.to_url()` only where a URL is still needed for display/logging. `resolve_playlist` in `youtube_api.rs` already takes a playlist ID, so its signature doesn't change.

### Testing approach
Integration tests live alongside the HTTP layer (e.g. `src/http/playlists.rs`'s `#[cfg(test)]` module, or `tests/playlists.rs` if a separate test binary reads better once there are several route files) and build the real `Router` from `serve.rs`'s router-construction function, parameterized with:
- `FakePlaylistRepository`: `Mutex<Vec<Playlist>>`, implementing the same trait.
- `FakeYoutubePlaylistLookup`: returns a caller-configured `bool` per test.
- `FixedClock`: returns a constant timestamp.

Requests are driven with `tower::ServiceExt::oneshot` against the router; assertions check HTTP status and JSON body (including exact `created_at` thanks to `FixedClock`). This requires adding `tower` (with the `util` feature for `oneshot`) as a dev-dependency, and `http-body-util` (or axum's own body-to-bytes helper) to read response bodies in tests.

### HTTP status mapping
- Create: `201 Created` with the playlist JSON on a new record; `200 OK` with the existing playlist JSON when the ID already exists (idempotent, no change made); `400 Bad Request` on invalid name or nonexistent YouTube ID.
- Delete: `204 No Content` on success; `400 Bad Request` if the ID doesn't exist (no change made).
- List: `200 OK` with a JSON array (possibly empty).

## Risks / Trade-offs

- [Existence-check adds an external call to every Create] -> acceptable since Create is a low-frequency, user-initiated action, not on any hot path; failure of the YouTube call surfaces as a clear 4xx/5xx rather than silently accepting a bad ID.
- [`created_at` stored as TEXT rather than an integer epoch] -> slightly more storage/parsing overhead, but human-readable when inspecting the SQLite file directly, which matches this project's low-traffic, single-table scale.
- [Trait objects over generics] -> minor dynamic-dispatch overhead, judged irrelevant at this request volume; kept for simpler handler signatures and test wiring.

## Open Questions

- Whether to add `chrono` as a new dependency for `DateTime<Utc>` handling, or hand-roll UTC timestamp formatting with `std::time::SystemTime` - doesn't affect the port's shape or the specs, safe to decide during implementation.
