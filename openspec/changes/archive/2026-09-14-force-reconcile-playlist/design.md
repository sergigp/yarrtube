## Context

`VideoService::reconcile_playlist(id)` (`src/domain/video/service.rs`) already
implements the full reconcile pass and is already called synchronously from
two places: `ReconcileOnPlaylistCreated` (an event subscriber, inline) and
`ReconcilePlaylistTask` (a background task handler). Both just call it and
propagate its `anyhow::Result<()>`. It already no-ops (returns `Ok(())`)
when the playlist no longer exists, and always reschedules the next
recurring reconcile at the end of a completed pass. See proposal.md for why
an on-demand trigger is needed.

An earlier version of this design reused `reconcile_playlist` unchanged for
the on-demand endpoint too, letting it also reschedule the next recurring
reconcile. That breaks under repeated on-demand triggering: each call
queues another pending `ReconcilePlaylist` task with nothing to remove the
older ones, so N clicks leave N pending tasks. A first fix attempt added a
`TaskRepository::cancel_pending(task_type, payload)` method to delete a
task's own duplicates before rescheduling — this pushed a domain-level
concern (what makes two `ReconcilePlaylist` tasks "the same") into the
generic repository layer, and its payload-equality check was subtly wrong
(comparing a JSON object payload against a raw string, which never
matches). The design below avoids the problem entirely instead of patching
around it.

## Goals / Non-Goals

**Goals:**
- Let a user trigger a reconcile pass for a specific playlist right now,
  from the web app, and see it reflected without waiting for the interval.
- Triggering it any number of times must never grow the number of pending
  `ReconcilePlaylist` tasks.
- Share the actual reconcile logic (membership diff + filesystem heal)
  with the existing recurring/creation path; do not duplicate it.

**Non-Goals:**
- A typed domain error enum for reconcile failures (unlike
  `CreatePlaylistError`/`DeletePlaylistError`). Reconcile failures are
  already logged and retried by the background path; the on-demand endpoint
  treats any failure generically.
- Async/polling semantics. The endpoint blocks until the pass completes.
- Any change to `TaskRepository` or to how the recurring/creation-triggered
  path schedules its next reconcile.

## Decisions

- **Split `reconcile_playlist` into two public methods sharing a private
  helper.** `VideoService` gains `force_reconcile_playlist(id)` alongside
  the existing `reconcile_playlist(id)`. Both look up the playlist
  (no-op if missing) and then call a new private `run_reconcile_pass`
  helper that holds the actual membership-diff-plus-filesystem-heal logic.
  `reconcile_playlist` additionally schedules the next recurring pass
  afterward, exactly as before; `force_reconcile_playlist` does not
  schedule anything at all. The on-demand HTTP handler calls
  `force_reconcile_playlist`. Because it never touches the task table, it
  cannot create duplicate pending tasks no matter how many times it's
  triggered, and it never disturbs whatever reconcile is already scheduled
  from creation or the last recurring pass. This keeps the "at most one
  pending reconcile per playlist" invariant as a structural property of the
  on-demand path rather than something a repository has to enforce after
  the fact, and requires no `TaskRepository` changes.
- **Synchronous endpoint.** `POST /playlists/{id}/reconcile` calls
  `video_service.force_reconcile_playlist(id)` inline, wrapped in
  `tokio::task::spawn_blocking` (matching `create_playlist`'s existing
  pattern for a blocking domain call), and only returns once the pass
  completes. Alternative considered: schedule a task and return
  immediately, requiring the UI to poll `/tasks` for completion - rejected
  as unnecessary complexity for a pass that normally completes in well
  under a second.
- **No existence check before calling reconcile.** `force_reconcile_playlist`
  already treats a missing playlist as a no-op (`Ok(())`), matching the
  recurring task's crash-safety behavior. The HTTP handler does not
  duplicate that check; a request for a nonexistent playlist ID gets the
  same success response as one that actually reconciled. This mirrors how
  the background task already behaves and avoids introducing a second
  source of truth for "does this playlist exist." In practice the web UI
  only ever calls this for a playlist ID it just rendered, so the race is
  narrow.
- **Generic error mapping.** Any `Err` from `force_reconcile_playlist`
  (YouTube API failure, filesystem error, etc.) maps to `500 Internal
  Server Error` with the error's message, the same fallback
  `create_playlist`'s handler uses for its own repository-error case. No
  new error taxonomy is introduced.
- **No confirmation dialog in the UI.** Unlike "Delete", "Reconcile" is
  non-destructive and idempotent, so the menu item triggers it directly.

## Risks / Trade-offs

- [A slow YouTube API call blocks the HTTP request for its duration] →
  Acceptable: `create_playlist` already has the same characteristic today,
  and `reconcile_playlist`'s YouTube call is a single playlist-items list
  request.
