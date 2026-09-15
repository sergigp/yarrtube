## Context

See `proposal.md` - Why. The codebase already has one aggregate shaped almost exactly like what's needed here: `Playlist` (`src/domain/playlist/`), which validates an external YouTube ID before persisting, is idempotent on that ID, and publishes create/delete domain events that downstream subscribers (`playlist-sync`, cascade-delete) react to. `Channel` follows the same shape and the same layering conventions (`rust-architect` skill): `domain/<aggregate>/` for orchestration and value objects, `http/<aggregate>/` as a thin adapter, `infrastructure/repositories/` for the SQLite store and the YouTube API port.

## Goals / Non-Goals

**Goals:**
- A `Channel` aggregate that can be created (idempotently), listed, and deleted through HTTP, following the exact `Playlist` pattern for validation, idempotency, and eventing.
- `ChannelCreated`/`ChannelDeleted` domain events published on every successful create/delete, ready for a future subscriber even though none is wired up in this change.
- SPA parity with the Playlists tab: list + create, no edit UI (no update endpoint exists to back one).

**Non-Goals:**
- Any sync or download behavior. `video_limit` is stored but nothing reads it yet.
- Refreshing a channel's stored `name` after creation (no update endpoint; a rename on YouTube's side is picked up only if the channel is deleted and re-added).
- Registering a subscriber or task handler for the new events, or adding new `SubscriberRegistry`/`HandlerRegistry` entries.
- Supporting legacy `/channel/UC...` channel URLs — only handle-based URLs and bare handles are accepted.

## Decisions

**`Channel` over `Subscription`/`ChannelSubscription` as the aggregate name.** `Playlist` is named after the YouTube resource it mirrors, not the relationship to it — despite `Playlist` already being something the daemon "subscribes to" and periodically syncs (`playlist-sync`). Naming the new aggregate `Channel` keeps that convention consistent and sets up a natural `channel-sync` capability name later, paralleling `playlist-sync`.

**Handle as `id`, immutable channel ID stored alongside it.** YouTube channel handles (`@name`) are owner-changeable; the underlying channel ID (`UC...`) is not. Using the handle as the primary/HTTP-facing ID keeps the API human-readable (`DELETE /channels/@somechannel`, matching how `PlaylistId` is often a readable slug), while storing `youtube_channel_id` separately gives a future sync capability something stable to key off even if the handle changes later. This trades a small amount of schema surface for avoiding a silent breakage mode.

**`name` is resolved from YouTube at creation time, not caller-supplied.** Unlike `Playlist.name` (freely chosen by the caller), `Channel.name` comes from the same `channels?forHandle=` API response used to validate the handle and resolve the immutable ID, so it costs nothing extra to capture. This intentionally diverges from `Playlist`'s request shape: `CreateChannelRequest` has no `name` field.

**New `YoutubeChannelRepository` port distinct from `YoutubePlaylistRepository`.** `YoutubePlaylistRepository::exists` only needs a boolean. Channel creation needs the resolved immutable ID and title too, so the port returns richer data (e.g. `resolve(handle) -> Option<{id, title}>`) rather than reusing `exists`'s boolean shape.

**No `PlaylistKind`-style enum.** `Playlist` has `kind` (`YoutubeLinked` | `Custom`) because two creation paths share one table. `Channel` has exactly one creation path, so no analogous enum is needed.

**Domain events published without a subscriber.** `domain-events`'s dispatch requirement already treats "no subscribers registered" as a normal, non-error outcome, so publishing `ChannelCreated`/`ChannelDeleted` now is safe and costs nothing at runtime; it's forward-provisioning for the sync capability that will consume them, per explicit scope from the proposal discussion.

**Reuse the shared `Quality` value object as-is.** It's already aggregate-agnostic (`domain/shared/quality.rs`); no changes needed there.

**`video_limit` validated as a positive integer at the HTTP boundary**, the same layer where `Playlist`'s quality/path parsing happens — reject non-numeric or non-positive values with a 400 before touching the repository or YouTube.

## Risks / Trade-offs

- **YouTube API dependency on create** → channel creation can fail on API outages/rate limits the same way playlist creation can; reuse the existing `Lookup` error class mapped to `502 Bad Gateway`, exactly as `CreatePlaylistError::Lookup` does.
- **Stale `name` after a channel rename** → accepted for this change since there's no update endpoint; re-adding the channel (delete + create) is the only way to refresh it today. Same limitation already exists implicitly for `Playlist.name` in spirit (it's never refreshed either).
- **`video_limit` is unused this iteration** → dead data until a sync capability reads it. Accepted per explicit scope decision; the field is cheap to add now and expensive to bolt on retroactively once real channels are already tracked.

## Migration Plan

Purely additive: new `channels` table, new routes, new SPA tab. No existing table, endpoint, or component is modified. Reverting is equivalent to dropping the new table and routes — no data migration or rollback coordination needed with existing capabilities.
