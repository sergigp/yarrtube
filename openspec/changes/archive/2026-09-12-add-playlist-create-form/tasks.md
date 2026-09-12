## 1. Backend: PlaylistId URL parsing

- [x] 1.1 Add `PlaylistId::from_url_or_id` in `src/domain/shared/playlist_id.rs`, mirroring `VideoId::from_url_or_id`: bare values pass through to `PlaylistId::new`; `https://`/`http://` values are matched against `youtube.com`, `www.youtube.com`, `m.youtube.com` hosts and the `list` query parameter is extracted regardless of path; anything else (unrecognized host, missing `list` param) is rejected. Verify with unit tests covering: bare ID, `youtube.com/playlist?list=...`, `youtube.com/watch?v=...&list=...`, extra query params, missing `list` param, unrecognized host, empty/whitespace value.

## 2. Backend: wire the parser into the create-playlist endpoint

- [x] 2.1 Rename `CreatePlaylistRequest.id` to `.playlist` in `src/http/playlists/dto.rs`.
- [x] 2.2 Update `create_playlist` in `src/http/playlists/mod.rs` to call `PlaylistId::from_url_or_id(request.playlist)` instead of `PlaylistId::new(request.id)`, and adjust the empty/missing-value error message to reference "playlist ID or URL". Verify with the existing `it_should_return_201_when_creating_a_new_playlist`-style tests updated to send `{"playlist": ...}`.
- [x] 2.3 Add HTTP-level tests for the new scenarios: creation from a full YouTube playlist URL, creation from a `watch?v=...&list=...` URL, unrecognized URL rejected with 400, YouTube URL missing `list` rejected with 400, empty `playlist` value rejected with 400 — verify via `cargo test playlists::`.

## 3. Spec & docs sync

- [x] 3.1 Confirm `openspec/specs/playlist-crud/spec.md`'s Create Playlist requirement matches the delta in this change once archived (no action needed now beyond the delta already written — checked at archive time via `openspec archive`).
- [x] 3.2 Update `README.md`'s `POST /api/playlists` row and its example `curl` command to use `{"playlist": ...}` and mention URL support.

## 4. Frontend: create-playlist API call

- [x] 4.1 Add `createPlaylist({ playlist, name, path, quality })` to `web/src/api.js`, POSTing JSON to `/api/playlists` and throwing on a non-OK response (including the response body's error message when present) the same way `request()` does for GET calls today.

## 5. Frontend: create-playlist form

- [x] 5.1 Add `web/src/components/CreatePlaylistForm.jsx`: controlled inputs for "Playlist ID or URL", name, path, and a quality `<select>` (`high`/`mid`/`low`), a submit button, and inline error display on failure. Verify by running the Vite dev server and submitting the form manually against a running daemon.
- [x] 5.2 Wire `CreatePlaylistForm` into `PlaylistsTab` in `web/src/App.jsx`, shown above `PlaylistList`. On successful creation, clear the form (the list picks up the new playlist on its next poll tick via existing `usePolling`).
- [x] 5.3 Add minimal styling for the form in `web/src/App.css`/`index.css` consistent with the existing list/tab styling.

## 6. End-to-end verification

- [x] 6.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`; all pass.
- [x] 6.2 Manually verify in the browser: create a playlist by pasting a full YouTube playlist URL, confirm it appears in the list; attempt an invalid URL and confirm a clear error is shown without crashing the form.
