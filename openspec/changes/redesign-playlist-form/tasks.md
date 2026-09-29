## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md's Files and Types & Signatures, wired end to end, with today's UI kept. **Rust:**
  - `YoutubePlaylistRepository::exists` becomes `resolve` → `Option<ResolvedPlaylist>`. The real adapter requests `part=snippet` and maps `snippet.title`, with `item_count: 0` as a placeholder, so smoke runs see real titles. `FakeYoutubePlaylistRepository { resolved }` replaces `{ exists }`.
  - `PlaylistName::from_youtube_title` returns the title unchanged (no fallback yet). Add `PlaylistPreview` and `PreviewPlaylistError`.
  - `PlaylistCreator::create(id, path, quality)` takes the name from `resolve_on_youtube`.
  - Add `PlaylistPreviewer` (`preview` returns `Err(YoutubePlaylistNotFound(id))`) to `domain/services`. Add it to `ApiServices` and `serve.rs`.
  - `CreatePlaylistRequest` drops `name`. Add `PreviewPlaylistQuery`, `PlaylistPreviewResponse`, and a `preview_playlist` handler that returns the previewer's error mapped to 404/502, routed at `GET /playlists/preview`.
  - Adapt the existing create tests and the "does not exist" adapter test to the new signatures. `it_should_create_a_playlist` expects the resolved title as the name.

  **Frontend** (the playlist dialog keeps its name field and destination box for now; the server ignores the `name` it still sends):
  - Add `previewPlaylist` to `api.js`. `createPlaylist` drops `name`.
  - Add `useDebouncedValue`, and `usePlaylistPreview` with the `playlistPreview` key.
  - Add `playlistNoticeLead`, returning a fixed string.
  - Extract `DestinationNotice.jsx` from `AddChannelDialog.jsx`, keeping the channel notice's behaviour.

  **Smoke:** set the `SMOKE_PLAYLIST_NAME` default in `scripts/run-smoke-tests.sh` to the smoke playlist's real YouTube title, since created playlists are now named after it.

  Done when:
  - `cargo build` and `cargo test --locked` pass.
  - `npm run build` and `npm run lint` pass in `web/`.
  - `scripts/run-smoke-tests.sh` passes with the existing specs.

## 2. Behaviour (TDD)

Rust cycles: red is an assertion failure, and green is `cargo test --locked` fully passing. Smoke cycles: green is `npm run build`, `npm run lint` and the full `scripts/run-smoke-tests.sh` passing.

- [x] 2.1 `it_should_name_the_playlist_after_its_id_if_youtube_title_blank`: when the YouTube title is blank, the created playlist is named after its ID.
- [ ] 2.2 `it_should_create_a_playlist_with_a_title_unsafe_for_filesystems`: a title such as "AC/DC: greatest hits?" is stored and listed back. This drives dropping the unsafe-character rule from `PlaylistName::new`, which the SQLite read path goes through.
- [ ] 2.3 `it_should_preview_a_playlist`: the preview returns the playlist's ID, title and video count and persists nothing.
- [ ] 2.4 `it_should_preview_a_playlist_from_a_youtube_url`: the ID is extracted from a watch URL carrying `list=`.
- [ ] 2.5 `it_should_fail_to_preview_if_invalid_playlist_provided`: a value that isn't a playlist is rejected with the value object's message without contacting YouTube.
- [ ] 2.6 `it_should_fail_to_preview_if_playlist_missing`: an absent `playlist` query parameter is rejected as empty.
- [ ] 2.7 `it_should_fail_to_preview_if_playlist_not_found_on_youtube`: a playlist YouTube doesn't know is reported as not found.
- [ ] 2.8 `it_should_fail_to_preview_if_youtube_lookup_fails`: a failed lookup is reported as a bad gateway.
- [ ] 2.9 `PlaylistName` › `it_should_accept_a_name_with_filesystem_unsafe_characters`: replaces the slash and backslash rejection tests.
- [ ] 2.10 `PlaylistName` › `it_should_name_after_the_id_if_youtube_title_blank`: `from_youtube_title` falls back to the ID.
- [ ] 2.11 `addPlaylistDialog.spec.js` › `it should show no notice until a playlist is entered`: the `destination-notice` element is absent and `Create Playlist` is disabled while the field is empty.
- [ ] 2.12 `addPlaylistDialog.spec.js` › `it should explain a value that is not a playlist`: the debounced preview's 400 message shows as an error notice, and submit stays disabled.
- [ ] 2.13 `addPlaylistDialog.spec.js` › `it should state the title, count and destination`:
  - Remove the `Name` field.
  - Feed `LocationField` the preview's title.
  - Move location and quality into the mounted-but-`hidden` "Advanced options".
  - Build the info notice from `playlistNoticeLead(video_count, title)` and the destination.
  - Remove `showDestination` and the destination box from `LocationField`, and adapt `addDialogLocation.spec.js` to assert through `destinationNotice` (see design.md Files).
  - `playlistNoticeLead` handles the counts 0, 1 and many.
- [ ] 2.14 `addPlaylistDialog.spec.js` › `it should expand advanced options from the change action`: `[change]` expands "Advanced options", and `Folder name` holds the slug of the title.
- [ ] 2.15 `playlist.spec.js` › lifecycle:
  - Add by ID only, and find the sidebar entry under the YouTube title.
  - Reopening with the same ID shows "Already added as" and disables `Create Playlist`: the dialog matches the preview's `id` against `usePlaylists()`.
  - The old path-conflict step is replaced.

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 `YoutubeApiPlaylistRepository` › `it_should_resolve_the_playlist_title_and_item_count`: request add `contentDetails` to `part` and map `contentDetails.itemCount` into `ResolvedPlaylist.item_count`, replacing the skeleton's placeholder 0.

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass.
- [ ] 4.2 `npm run build` and `npm run lint` pass in `web/`, and `scripts/run-smoke-tests.sh` is fully green.
- [ ] 4.3 Manual check with `scripts/run-local.sh` at desktop width and at about 375px:
  - The playlist dialog shows only the ID/URL field until something is typed. It then shows "Looking up playlist…", followed by the count, the title and the path.
  - A folder edit updates the notice, and `[change]` expands "Advanced options".
  - A bad URL, an unknown ID, an already-added playlist and a taken folder each show an error notice with submit disabled.
  - A title containing `/` or `:` creates a playlist listed under that exact name.
  - The channel dialog behaves as before.
