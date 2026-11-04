# Design

## Files

Rust
- `migrations/0022_source_summaries.sql` — new: `summaries_enabled INTEGER NOT NULL DEFAULT 0` on `channels` and `playlists`.
- `src/infrastructure/shared/sqlite_migrations.rs` — register `0022`.
- `src/domain/preferences/preferences.rs` — `summaries_available()`: AI and summaries both on.
- `src/domain/channel/errors.rs`, `src/domain/playlist/errors.rs` — `SummariesUnavailable` on create/update errors.
- `src/domain/channel/channel.rs` — `summaries_enabled` field (`false` in `create`), `with_summaries_enabled`.
- `src/domain/channel/channel_settings_change.rs` — new: partial update, mirrors `PlaylistSettingsChange`.
- `src/domain/channel/channel_view.rs` — `summaries_enabled`.
- `src/domain/channel/mod.rs` — export `ChannelSettingsChange`.
- `src/domain/playlist/playlist.rs` — `summaries_enabled` field (`false` in `create`), `with_summaries_enabled`.
- `src/domain/playlist/playlist_settings_change.rs` — `summaries_enabled: Option<bool>`; `is_empty` covers it.
- `src/domain/playlist/playlist_view.rs` — `summaries_enabled`.
- `src/domain/services/channel_creator.rs`, `playlist_creator.rs` — `create` takes `summaries_enabled: bool`; inject `PreferencesRepository`, check before anything else.
- `src/domain/services/channel_updater.rs` — `update_settings(id, ChannelSettingsChange)`; inject `PreferencesRepository`, check before loading the channel.
- `src/domain/services/playlist_updater.rs` — apply `summaries_enabled`; inject `PreferencesRepository`, same check.
- `src/serve.rs` — pass `infrastructure.preferences_repository` to the four services.
- `src/domain/services/channel_view_searcher.rs`, `playlist_view_searcher.rs` — copy the field into the view.
- `src/infrastructure/repositories/sqlite_channel_repository.rs`, `sqlite_playlist_repository.rs` — read/insert/update the column.
- `src/application/http/channels/{mod,dto}.rs`, `playlists/{mod,dto}.rs` — request field on create/update, response field on every response DTO; `SummariesUnavailable` → 409.

Web
- `web/src/api/types.ts` — `summaries_enabled` on `ChannelListItem`, `PlaylistListItem`, `CreatedChannel`.
- `web/src/api/client.ts` — `summaries_enabled` on `Create{Channel,Playlist}Request`, `summaries_enabled?` on `Update{Channel,Playlist}Request`.
- `web/src/test/helpers.tsx` — `aChannel`/`aPlaylist` default `summaries_enabled: false`.
- `web/src/lib/preferences.ts` — `summariesAvailable(preferences)`.
- `web/src/api/queries.ts` — `useSummariesAvailable()`.
- `web/src/lib/channelSettings.ts` — form + changes include `summaries_enabled`.
- `web/src/components/SummariesField.tsx` — new: checkbox, hint, disabled state with `/ai` link.
- `web/src/components/{AddChannelDialog,AddPlaylistDialog,EditChannelDialog,EditPlaylistDialog}.tsx` — render `SummariesField`, send the value; in add dialogs below "Save to"/categories, outside "Advanced options".

## Types & Signatures

```rust
// domain/channel/channel.rs (same on Playlist)
pub struct Channel { /* … */ pub summaries_enabled: bool }
impl Channel {
    pub fn with_summaries_enabled(self, summaries_enabled: bool) -> Self;
}

// domain/preferences/preferences.rs
impl Preferences { pub fn summaries_available(&self) -> bool; } // ai_enabled && summaries.enabled

// domain/channel/errors.rs (same variant on CreatePlaylistError, UpdatePlaylistError)
pub enum CreateChannelError { /* … */
    #[error("Turn on AI summaries before enabling them for a channel")] SummariesUnavailable }
pub enum UpdateChannelError { /* … */
    #[error("Turn on AI summaries before enabling them for a channel")] SummariesUnavailable }

// services gain a dependency (ChannelCreator, ChannelUpdater, PlaylistCreator, PlaylistUpdater)
preferences_repository: Arc<dyn PreferencesRepository>,
// private helper in each: Err(SummariesUnavailable) when enabling and
// !preferences.unwrap_or_default().summaries_available()
fn ensure_summaries_available(&self, summaries_enabled: bool) -> Result<(), …>;

// domain/channel/channel_settings_change.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChannelSettingsChange {
    pub quality: Option<Quality>,
    pub video_limit: Option<VideoLimit>,
    pub reconcile_interval: Option<ReconcileInterval>,
    pub summaries_enabled: Option<bool>,
}
impl ChannelSettingsChange { pub fn is_empty(&self) -> bool; }

// domain/playlist/playlist_settings_change.rs
pub struct PlaylistSettingsChange { /* … */ pub summaries_enabled: Option<bool> }

// ChannelView / PlaylistView
pub summaries_enabled: bool,

// services
fn create(&self, id: ChannelHandle, quality: Quality, video_limit: VideoLimit,
          reconcile_interval: ReconcileInterval, path: PlaylistPath,
          summaries_enabled: bool) -> Result<CreateChannelOutcome, CreateChannelError>;
fn create(&self, id: PlaylistId, path: PlaylistPath, quality: Quality, exclude_from_home: bool,
          reconcile_interval: ReconcileInterval,
          summaries_enabled: bool) -> Result<CreatePlaylistOutcome, CreatePlaylistError>;
fn update_settings(&self, id: ChannelHandle, change: ChannelSettingsChange)
    -> Result<Channel, UpdateChannelError>;

// dto.rs: Create*/Update*Request
#[serde(default)] pub summaries_enabled: Option<bool>,
// ChannelResponse, ChannelListItemResponse, PlaylistResponse, PlaylistListItemResponse
pub summaries_enabled: bool,
```

```ts
// lib/preferences.ts — both AI and summaries on; false while loading/failed
export function summariesAvailable(preferences: Preferences | undefined): boolean
// api/queries.ts
export function useSummariesAvailable(): boolean
// lib/channelSettings.ts
interface ChannelSettingsForm { /* … */ summaries_enabled: boolean }
// components/SummariesField.tsx
interface SummariesFieldProps {
  id: string
  checked: boolean
  onChange: (checked: boolean) => void
  /** The source's stored value: when true the box stays enabled so summaries can be turned off. */
  enabledOnSource?: boolean
  /** Called when the "AI settings" link is followed, to close the dialog. */
  onNavigateAway: () => void
}
```

## Call Stack

Create: `POST /api/channels {…, summaries_enabled}` → `create_channel` (`request.summaries_enabled.unwrap_or(false)`) → `ChannelCreator::create(…, summaries_enabled)` → `ensure_summaries_available(summaries_enabled)` (`PreferencesRepository::find()`; `Err(SummariesUnavailable)` → 409) → `find_existing` → `resolve_on_youtube` → `Channel::create(…).with_summaries_enabled(summaries_enabled)` → `ChannelRepository::insert(&channel)` → `ChannelResponse::from(channel)`. Playlist: same through `create_playlist` → `PlaylistCreator::create`.

Update: `PATCH /api/channels/:id {summaries_enabled}` → `update_channel` builds `ChannelSettingsChange` (VOs via `?`), `is_empty()` → 400 `NOTHING_TO_UPDATE` → `ChannelUpdater::update_settings(id, change)` → `ensure_summaries_available(change.summaries_enabled == Some(true))` (409) → `find_channel` → `apply_settings(channel, change)` (`with_summaries_enabled`) → `ChannelRepository::update(&channel)` → `bring_forward_next_reconcile` only if `change.reconcile_interval`. Playlist: `update_playlist` → `PlaylistSettingsChange { summaries_enabled, … }` → `PlaylistUpdater::update_settings`.

List: `GET /api/channels` → `ChannelViewSearcher` → `ChannelView { summaries_enabled: channel.summaries_enabled, … }` → `ChannelListItemResponse`.

UI: dialog → `<SummariesField checked enabledOnSource …/>` → `useSummariesAvailable()` (`usePreferences` → `summariesAvailable(data)`), disabled when `!available && !enabledOnSource`; edit dialogs pass the source's stored value; link `<Link to="/ai" onClick={onNavigateAway}>`. Add dialogs send `summaries_enabled` in `createChannel`/`createPlaylist`; edit dialogs send it only when it differs from the source's.

## Test Plan

### 1. Behaviour tests

`application/http/channels` (acceptance, real SQLite)
1. `it_should_create_a_channel_with_summaries_disabled_by_default` — omitted → stored and returned `summaries_enabled: false`.
2. `it_should_create_a_channel_with_summaries_enabled` — preferences seeded AI + summaries on; `true` → stored and returned `true`.
3. `it_should_fail_to_create_a_channel_with_summaries_if_ai_is_off` — default preferences; 409 message, nothing stored, YouTube fake not called.
4. `it_should_enable_summaries_on_a_channel` — AI + summaries on; only `summaries_enabled: true` → 200, stored `true`, other fields unchanged.
5. `it_should_fail_to_enable_summaries_on_a_channel_if_summaries_are_off` — AI on, summaries off; with a new quality → 409, channel unchanged.
6. `it_should_disable_summaries_on_a_channel_while_ai_is_off` — default preferences, channel seeded on → stored `false`.
7. `it_should_list_channels_with_their_summaries_setting` — seeded one on, one off → list items carry each value.

`application/http/playlists` (acceptance, real SQLite)
8. `it_should_create_a_playlist_with_summaries_disabled_by_default`.
9. `it_should_create_a_playlist_with_summaries_enabled` — AI + summaries on.
10. `it_should_fail_to_create_a_playlist_with_summaries_if_ai_is_off` — 409, nothing stored, YouTube fake not called.
11. `it_should_enable_summaries_on_a_playlist` — AI + summaries on; only `summaries_enabled: true` → 200, stored `true`, others unchanged.
12. `it_should_fail_to_enable_summaries_on_a_playlist_if_summaries_are_off` — AI on, summaries off; with a new interval → 409, playlist unchanged.
13. `it_should_disable_summaries_on_a_playlist_while_ai_is_off`.
14. `it_should_list_playlists_with_their_summaries_setting`.

`domain/preferences/preferences.rs` (unit)
15. `it_should_have_summaries_available_only_with_ai_and_summaries_on` — all four combinations.

Web (Vitest)
16. `preferences.test.ts` — `summariesAvailable`: true only with `ai_enabled` and `summaries_enabled`; false for `undefined`.
17. `channelSettings.test.ts` — `channelSettingsChanges` includes `summaries_enabled` only when changed.
18. `AddChannelDialog.test.tsx` — AI summaries on: "Summarize videos" visible without expanding "Advanced options"; checking it sends `summaries_enabled: true`; untouched sends `false`.
19. `AddPlaylistDialog.test.tsx` — same as 18 for playlists.
20. `AddChannelDialog.test.tsx` — AI off: checkbox disabled, hint "Turn on AI summaries in AI settings." with link to `/ai`; following it closes the dialog and routes to `/ai`.
21. `EditChannelDialog.test.tsx` — prefilled checked; AI on: toggling sends `summaries_enabled` alone and triggers no sync.
22. `EditChannelDialog.test.tsx` — AI on, summaries off: checked and enabled for a source with it on; unchecking sends `summaries_enabled: false` alone.
23. `EditPlaylistDialog.test.tsx` — AI on: checking sends `{ summaries_enabled: true }` alone; AI off: disabled, and enabled for a source with summaries on.

### 2. Infrastructure tests

`sqlite_channel_repository`
24. `it_should_round_trip_the_channels_summaries_setting` — insert `true`, find → `true`; update to `false` → `false`.

`sqlite_playlist_repository`
25. `it_should_round_trip_the_playlists_summaries_setting`.

`sqlite_migrations`
26. `it_should_disable_summaries_for_sources_stored_before_the_setting` — rows inserted before `0022` read `summaries_enabled = 0`.
