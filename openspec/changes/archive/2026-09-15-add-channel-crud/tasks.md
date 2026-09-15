## 1. Domain: Channel aggregate

- [x] 1.1 Add `src/domain/channel/mod.rs`, `channel.rs` (the `Channel` struct: `id`, `name`, `youtube_channel_id`, `quality`, `video_limit`, `created_at`), mirroring `src/domain/playlist/playlist.rs`, and verify `cargo build` compiles the new module
- [x] 1.2 Add `src/domain/channel/channel_handle.rs`: a `ChannelHandle` value object requiring a non-empty value starting with `@`, plus a `from_url_or_handle` parser accepting a bare handle or a `youtube.com/@handle` URL (reject legacy `/channel/UC...` URLs and unrecognized hosts), mirroring `src/domain/shared/playlist_id.rs::from_url_or_id`; verify with unit tests covering each spec scenario (bare handle, URL, missing `@`, unrecognized URL, URL without a handle, empty value)
- [x] 1.3 Add `src/domain/channel/video_limit.rs`: a `VideoLimit` value object rejecting non-positive values; verify with unit tests for a valid limit and a zero/negative rejection
- [x] 1.4 Add `src/domain/channel/errors.rs` (`CreateChannelError` with `YoutubeChannelNotFound`, `Lookup`, `Repository` variants; `DeleteChannelError` with `NotFound`, `Repository`), mirroring `src/domain/playlist/errors.rs`

## 2. Infrastructure: YouTube channel resolution

- [x] 2.1 Add `src/infrastructure/repositories/youtube_channel_repository.rs`: a `YoutubeChannelRepository` trait with `resolve(&self, handle: &ChannelHandle) -> anyhow::Result<Option<ResolvedChannel>>` (`ResolvedChannel { youtube_channel_id, title }`), a `YoutubeApiChannelRepository` implementation calling `GET https://www.googleapis.com/youtube/v3/channels?part=id,snippet&forHandle=<handle>`, and a `FakeYoutubeChannelRepository` for tests, mirroring `src/infrastructure/repositories/youtube_playlist_repository.rs`
- [x] 2.2 Verify with unit tests (using `mockito`, matching the existing `youtube_playlist_repository.rs` test style) covering a handle that resolves and one that returns no items

## 3. Infrastructure: Channel storage

- [x] 3.1 Add `src/infrastructure/repositories/sqlite_channel_repository.rs`: a `ChannelRepository` trait (`find`, `insert`, `delete`, `list`), a `SqliteChannelRepository` creating a `channels` table (`id`, `name`, `youtube_channel_id`, `quality`, `video_limit`, `created_at`), and a `FakeChannelRepository` for tests, mirroring `src/infrastructure/repositories/sqlite_playlist_repository.rs`
- [x] 3.2 Verify with unit tests covering find-missing, insert-then-find, duplicate-insert failure, delete, delete-missing, list-empty, and list-all, mirroring the existing `sqlite_playlist_repository.rs` test suite

## 4. Domain: Channel service and events

- [x] 4.1 Add `ChannelCreated { channel_id: String }` and `ChannelDeleted { channel_id: String }` variants to `DomainEvent` in `src/domain/event.rs`, and verify `cargo build` compiles with the new variants handled everywhere `DomainEvent` is matched exhaustively
- [x] 4.2 Add `src/domain/channel/service.rs`: `ChannelService::create_channel` (find-existing idempotency check, resolve handle via `YoutubeChannelRepository`, persist, publish `ChannelCreated` only on new creation), `delete_channel` (find-or-`NotFound`, delete, publish `ChannelDeleted`), and `list_channels`, mirroring `src/domain/playlist/service.rs`
- [x] 4.3 Verify with unit tests covering every scenario in `openspec/changes/add-channel-crud/specs/channel-crud/spec.md`: successful creation (bare handle and URL), idempotent duplicate (ignoring a different quality/video_limit), nonexistent YouTube channel, successful deletion, deleting a nonexistent channel, `ChannelCreated` published once per new channel and not on duplicates, `ChannelDeleted` published on deletion and not for a missing channel

## 5. HTTP: Channel endpoints

- [x] 5.1 Add `src/http/channels/dto.rs` (`CreateChannelRequest { channel, quality, video_limit }`, `ChannelResponse`) and `src/http/channels/mod.rs` (`create_channel`, `delete_channel`, `list_channels` handlers), mirroring `src/http/playlists/{dto.rs,mod.rs}` for request parsing, validation-before-YouTube-call ordering, and status code mapping (`201`/`200` on create, `204` on delete, `400` for validation/not-found/nonexistent-channel, `502` for `Lookup`, `500` for `Repository`)
- [x] 5.2 Register `POST /channels`, `GET /channels`, `DELETE /channels/{handle}` in `api_router` (`src/http/mod.rs`), add `channel_service: ChannelService` to `AppState`
- [x] 5.3 Verify with `axum::Router` integration tests (mirroring `src/http/playlists/mod.rs`'s test module) covering every HTTP-level scenario in the `channel-crud` spec: 201/200 create responses, 400s for each validation and not-found case, 502 for a YouTube lookup failure, 204 delete, 400 delete-missing, and empty/populated list responses

## 6. Composition root

- [x] 6.1 Wire `SqliteChannelRepository`, `YoutubeApiChannelRepository`, and `ChannelService` into `serve.rs::build_application()`, and verify `cargo build --release` succeeds and `./target/release/yarrtube serve` starts without error against a fresh database

## 7. SPA

- [x] 7.1 Add `fetchChannels`, `createChannel`, `deleteChannel` to `web/src/api.js`, mirroring the existing `fetchPlaylists`/`createPlaylist`/`deletePlaylist` functions
- [x] 7.2 Add `web/src/components/ChannelList.jsx` (mirroring `PlaylistList.jsx`, showing name, quality, and video limit) and `web/src/components/CreateChannelForm.jsx` (mirroring `CreatePlaylistForm.jsx`, fields: channel handle/URL, quality, video limit — no name field, since it's resolved server-side)
- [x] 7.3 Add a "Channels" tab to `web/src/App.jsx` alongside Playlists/Tasks, wiring in `ChannelList` and a create-channel dialog analogous to `AddPlaylistDialog.jsx`
- [x] 7.4 Verify by running the dev server (`npm run dev` in `web/`) against a local `yarrtube serve`, creating a channel through the UI, confirming it appears in the list, and deleting it

## 8. Full verification

- [x] 8.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`, and verify all three pass
