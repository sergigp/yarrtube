## 1. Domain layer

- [x] 1.1 Add `YoutubePlaylistId` value object (non-empty validation, `to_url()`) with unit tests covering valid/empty input and URL construction
- [x] 1.2 Add `PlaylistName` value object (non-empty, rejects filesystem-unsafe characters) with unit tests covering valid names, empty names, and names containing `/` and `\`
- [x] 1.3 Add `Playlist` entity (`id`, `name`, `created_at`) with a `create` constructor, and unit tests verifying it only builds from already-valid value objects
- [x] 1.4 Define `PlaylistRepository`, `YoutubePlaylistLookup`, and `Clock` port traits (dyn-compatible) and verify the crate compiles with no implementations yet (traits only)

## 2. Infrastructure adapters

- [x] 2.1 Implement `SqlitePlaylistRepository` (rusqlite) with a `CREATE TABLE IF NOT EXISTS` for playlists (`id TEXT PRIMARY KEY`, `name TEXT`, `created_at TEXT`), returning `SaveOutcome::Created` or, on a primary-key violation, re-selecting and returning `SaveOutcome::AlreadyExisted` (idempotent create, no change made), verified by unit tests against a temporary/in-memory SQLite file covering save, duplicate save (returns the original unchanged record), delete (found/not found), and list (empty/non-empty)
- [x] 2.2 Implement `YoutubePlaylistLookup` via the YouTube Data API v3 `playlists` resource (`GET /playlists?id=...&part=id`), returning whether `items` is non-empty, verified by unit tests against a mocked HTTP response for both the exists and not-exists cases
- [x] 2.3 Implement `SystemClock` (`Clock` returning the current UTC time) and a `FixedClock` test double returning a constant timestamp, verified by a unit test asserting `FixedClock::now()` returns the configured value

## 3. HTTP layer

- [x] 3.1 Add request/response DTOs and the `POST /playlists` handler: parses body, calls `YoutubePlaylistLookup::exists`, constructs `Playlist` via `Clock::now()`, calls `PlaylistRepository::save`, and maps outcomes to `201` (created) / `200` (already existed, idempotent) / `400` (invalid name or nonexistent YouTube ID), verified by the integration tests in section 5
- [x] 3.2 Add the `DELETE /playlists/{id}` handler mapping `PlaylistRepository::delete` outcomes to `204`/`400` (not found), verified by the integration tests in section 5
- [x] 3.3 Add the `GET /playlists` handler returning the repository's list as JSON (including the empty-list case), verified by the integration tests in section 5
- [x] 3.4 Wire the new routes and concrete adapters (`SqlitePlaylistRepository`, `YoutubePlaylistLookup` impl, `SystemClock`) into the router/state built in `src/serve.rs`, verified by `cargo build` succeeding and `cargo run -- serve` starting without error

## 4. CLI refactor

- [x] 4.1 Change the `download` CLI argument from a playlist URL to a `YoutubePlaylistId`, removing `extract_playlist_id`, verified by updating/passing the existing unit tests in `src/youtube_api.rs` and `src/download_command.rs`
- [x] 4.2 Update `README.md`'s `download` usage examples to pass a playlist ID instead of a URL, verified by re-reading the section for consistency

## 5. Integration tests

- [x] 5.1 Add `tower` (with `util`) as a dev-dependency and a test helper that builds the real router from `src/serve.rs` parameterized with fake adapters, verified by `cargo test` compiling the new test module
- [x] 5.2 Add `FakePlaylistRepository` and `FakeYoutubePlaylistLookup` test doubles implementing the port traits, verified by unit tests of the fakes themselves (save-then-list, save-then-delete)
- [x] 5.3 Add integration tests for `POST /playlists` covering: successful creation (201, body matches `FixedClock` timestamp), duplicate ID (200, returns the existing unchanged record), invalid name (400), nonexistent YouTube ID (400)
- [x] 5.4 Add integration tests for `DELETE /playlists/{id}` covering: successful deletion (204) and deleting a nonexistent ID (400)
- [x] 5.5 Add integration tests for `GET /playlists` covering: empty list (200, `[]`) and a populated list (200, all created playlists returned)

## 6. Verification

- [x] 6.1 Run `cargo fmt --check`, `cargo clippy`, and `cargo test` and confirm all pass
