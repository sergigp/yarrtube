## Context

`VideoService::reconcile_playlist(id)` (`src/domain/video/service.rs`) already
implements the full reconcile pass and is already called synchronously from
two places: `ReconcileOnPlaylistCreated` (an event subscriber, inline) and
`ReconcilePlaylistTask` (a background task handler). Both just call it and
propagate its `anyhow::Result<()>`. It already no-ops (returns `Ok(())`)
when the playlist no longer exists, and always reschedules the next
recurring reconcile at the end of a completed pass. See proposal.md for why
an on-demand trigger is needed.

## Goals / Non-Goals

**Goals:**
- Let a user trigger a reconcile pass for a specific playlist right now,
  from the web app, and see it reflected without waiting for the interval.
- Reuse `reconcile_playlist` as-is; add no new domain logic.

**Non-Goals:**
- Task deduplication/cancellation of the previously scheduled recurring
  reconcile. The forced pass reschedules the next one from its own
  completion time; the older schedule is simply left to fire later too
  (harmless, since reconcile is idempotent).
- A typed domain error enum for reconcile failures (unlike
  `CreatePlaylistError`/`DeletePlaylistError`). Reconcile failures are
  already logged and retried by the background path; the on-demand endpoint
  treats any failure generically.
- Async/polling semantics. The endpoint blocks until the pass completes.

## Decisions

- **Synchronous endpoint.** `POST /playlists/{id}/reconcile` calls
  `video_service.reconcile_playlist(id)` inline, wrapped in
  `tokio::task::spawn_blocking` (matching `create_playlist`'s existing
  pattern for a blocking domain call), and only returns once the pass
  completes. Alternative considered: schedule a task and return
  immediately, requiring the UI to poll `/tasks` for completion - rejected
  as unnecessary complexity for a pass that normally completes in well
  under a second.
- **No existence check before calling reconcile.** `reconcile_playlist`
  already treats a missing playlist as a no-op (`Ok(())`), matching the
  recurring task's crash-safety behavior. The HTTP handler does not
  duplicate that check; a request for a nonexistent playlist ID gets the
  same success response as one that actually reconciled. This mirrors how
  the background task already behaves and avoids introducing a second
  source of truth for "does this playlist exist." In practice the web UI
  only ever calls this for a playlist ID it just rendered, so the race is
  narrow.
- **Generic error mapping.** Any `Err` from `reconcile_playlist` (YouTube API
  failure, filesystem error, etc.) maps to `500 Internal Server Error` with
  the error's message, the same fallback `create_playlist`'s handler uses
  for its own repository-error case. No new error taxonomy is introduced.
- **No confirmation dialog in the UI.** Unlike "Delete", "Reconcile" is
  non-destructive and idempotent, so the menu item triggers it directly.

## Risks / Trade-offs

- [A slow YouTube API call blocks the HTTP request for its duration] →
  Acceptable: `create_playlist` already has the same characteristic today,
  and `reconcile_playlist`'s YouTube call is a single playlist-items list
  request.
- [Forcing a reconcile leaves a redundant future reconcile scheduled from
  before] → Accepted per proposal.md; reconcile is idempotent, so the extra
  pass just re-verifies state and reschedules again from itself.
