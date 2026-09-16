## Context

See proposal.md for motivation. Two existing constraints shape this design:

- `add-channel-video-sync`'s design.md deliberately made `DownloadVideo`/`DeleteVideoFile` task payloads container-agnostic ("Tasks stay single and container-agnostic") so handlers don't depend on `PlaylistRepository`/`ChannelRepository`. `download_video`'s payload carries only `video_id` (a `VideoRecordId`), never a `playlist_id`/`channel_id`.
- This repo's layering rule (`CLAUDE.md`, `rust-architect` skill): `application/http` is a thin adapter; orchestration belongs in `domain/services/`. Today's `list_tasks` handler violates this directly — it loops over tasks and calls `playlist_searcher`/`channel_service` itself.

## Goals / Non-Goals

**Goals:**
- Enrich every non-completed task type with whatever context is genuinely resolvable from its stored payload, not just `reconcile_playlist`/`reconcile_channel`.
- Move that orchestration into a domain service, leaving the HTTP handler a single call + response mapping.
- Preserve the container-agnostic task payload decision — no write-path changes to `Task`/`ScheduledTask`/task handlers.

**Non-Goals:**
- Not exposing a video's YouTube id (`VideoId`) on the enriched view — title alone is sufficient (confirmed).
- Not resolving `delete_playlist_files`/`delete_channel_files`'s container name — the container row is gone by the time the task is queryable, so a live join returns nothing; a name would require snapshotting it onto the payload at schedule time, which is a write-path change out of scope here.
- Not adding a `video_id`/any id to `delete_video_file`'s view — its payload structurally has none to resolve from.

## Decisions

### A dedicated `TaskViewSearcher`, not a fatter `TaskService`
New domain service in `domain/services/task_view_searcher.rs` with `search_all_pending() -> anyhow::Result<Vec<TaskView>>`, mirroring the existing `PlaylistSearcher`/`VideoSearcher` naming and shape (a read-oriented `*Searcher` injected with every port it needs to join across). `TaskService` (`domain/task/service.rs`) stays as-is — it's the scheduling-facing facade (`list_non_completed`) other domain code (subscribers, task handlers) depends on; bolting five more repository dependencies onto it for an HTTP-only read-model would conflate two different concerns.

### `TaskView.payload: HashMap<String, String>`, not a fixed struct or an enum
```rust
pub struct TaskView {
    pub id: i64,
    pub task_type: String,
    pub status: TaskStatus,
    pub retries: i64,
    pub run_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub payload: HashMap<String, String>,
}
```
A fixed struct (today's `playlist_name`/`channel_name` fields) is exactly the problem being fixed — most fields are `null` for most task types. A Rust enum (`TaskViewPayload::DownloadVideo { video_title, container_name }`, etc.) would be more type-safe but pushes per-variant flattening into the HTTP serialization layer for no real benefit, since the frontend already switches on `task_type` to interpret the payload (mirroring how `Task`'s own write-side payload is an opaque, type-keyed JSON blob). Chose the map for symmetry with that existing pattern; the trade-off (no compiler-enforced key presence) is covered by one unit test per `task_type` asserting its expected keys.

Per-`task_type` keys:
```
reconcile_playlist     -> { playlist_name }
reconcile_channel      -> { channel_name }
download_video         -> { video_title, playlist_name }  or  { video_title, channel_name }
delete_video_file      -> { filename }
delete_playlist_files  -> { path }
delete_channel_files   -> { path }
update_ytdlp            -> {}
```
A key is omitted (not present with an empty value) when it can't be resolved — e.g. `reconcile_playlist` for a playlist that's since been deleted yields `{}`, matching today's "omit the name" behavior for a missing reference.

### New `find_by_video` on `PlaylistVideoRepository`/`ChannelVideoRepository`, not a widened `download_video` payload
```rust
fn find_by_video(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<PlaylistVideo>>;
// ChannelVideoRepository gets the equivalent, returning Option<ChannelVideo>
```
Both tables are already keyed with a plain `video_id` column (`PlaylistVideo.video_id: VideoRecordId`), so this is a one-line query, mirroring the existing `find_by_youtube_video` method shape on each trait. `TaskViewSearcher` tries the playlist repository first; if that misses, tries the channel repository. This is the only way to resolve `download_video`'s container without reopening the container-agnostic payload decision.

### `delete_video_file`/`delete_playlist_files`/`delete_channel_files` get no repository calls at all
Their payload fields (`filename`, or `path`) are copied straight into `TaskView.payload` with no lookup:
- `delete_video_file`'s payload has no id to resolve anything from (video row is typically already gone by the time it's scheduled).
- `delete_playlist_files`/`delete_channel_files` do carry `playlist_id`/`channel_id`, but the container is deleted by the time the task exists — a live lookup would resolve to `None` almost every time. `path` already identifies the target for display purposes.

### `list_tasks` thins to one domain call
```rust
pub async fn list_tasks(State(state): State<AppState>) -> Response {
    let views = state.task_view_searcher.search_all_pending()?;
    (StatusCode::OK, Json(views.into_iter().map(TaskResponse::from).collect())).into_response()
}
```
`playlist_searcher`/`channel_service` remain on `AppState` for the endpoints that already use them directly; they're just no longer called from `tasks/mod.rs`.

## Risks / Trade-offs

- [Risk] A video could in principle be tracked by both a playlist (via the custom-playlist add flow) and a channel simultaneously, making `find_by_video`'s playlist-then-channel probe ambiguous → Mitigation: this is a strict improvement over today (which shows nothing for `download_video` regardless); accept that the rare dual-membership case may show one container over the other, since no task-scheduling code path handles that ambiguity differently either.
- [Risk] `payload: HashMap<String, String>` gives up compile-time key guarantees → Mitigation: one test per `task_type` asserting its exact expected key set.
- [Risk] Response shape change is **BREAKING** for the SPA → Mitigation: SPA and daemon are built and deployed together (the daemon serves its own bundled SPA), so both sides land in the same change with no separate versioning/rollout concern.
