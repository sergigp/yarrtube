## Context

See proposal.md - Why. Relevant current state:

- The runtime image (`debian:bookworm-slim`) contains `yarrtube`, `/app/bin/yt-dlp` (the standalone `yt-dlp_linux` build, which already bundles the `yt_dlp_ejs` challenge-solver scripts) and ffmpeg. yt-dlp probes `PATH` for a JS runtime and only enables Deno by default.
- `PlaylistVideoReconciler` and `ChannelVideoReconciler` each have a loop over `stored_videos` filtered by `status == Errored` that calls `Video::reset_for_redownload` and schedules a `DownloadVideo` task. It runs on every reconcile pass (hourly per playlist/channel, plus on-demand and on-create passes).
- `Video` has no dedicated "errored at" timestamp. `mark_errored` sets `updated_at = now`, but `updated_at` can't anchor a cooldown: every reconcile pass (playlist `sync_playlist_membership` and the channel equivalent) rewrites each current member with `updated_at: now` while refreshing its title, before the errored-video recovery step runs.
- `SqliteTaskRepository::list_eligible` selects `status = 'pending' AND run_at <= now ORDER BY id ASC`. `TaskExecutor::poll_once` runs that snapshot serially. `run_at` is stored as RFC 3339 text in UTC, and the eligibility filter already relies on its lexical order.

## Goals / Non-Goals

**Goals:**
- yt-dlp inside the container detects a JS runtime with no extra flags.
- A video that keeps failing is retried at most about once a day by reconcile, not every hour.
- Tasks due earlier are dispatched first.

**Non-Goals:**
- No hard cap or manual "give up" state for errored videos, and no retry endpoint.
- No concurrency in the task executor.
- No cleanup of tasks already queued on a running deployment (see Migration Plan).
- No change to per-task retry/backoff/dead-letter behaviour.

## Decisions

**Deno from the official `denoland/deno:bin` image.** Use `COPY --from=denoland/deno:bin /deno /usr/local/bin/deno` in the runtime stage. The image is multi-arch (amd64/arm64), matching the existing `TARGETARCH` support. The binary is glibc-linked and runs on bookworm. Verified: in a bookworm-slim image with the same yt-dlp build, `yt-dlp -v` reports `JS runtimes: deno-…` and a video that failed on the NAS with `This video is not available` resolves as `public`.
- *Alternatives:* installing Node via apt (bookworm's Node is older, and yt-dlp needs `--js-runtimes node` since only Deno is on by default); downloading a Deno zip with curl in the `ytdlp-fetch` stage (needs unzip and a per-arch URL, duplicating what the official image already provides).

**Cooldown is measured from a new `last_errored_at` timestamp.** Add `Video.last_errored_at: Option<DateTime<Utc>>`, backed by a nullable `videos.last_errored_at` column (new migration). `mark_errored` sets it to `now`, and `reset_for_redownload` keeps it, so it always records the last time the video permanently failed. A domain predicate on `Video`, `is_due_for_recovery(now) -> bool`, is true when `status == Errored` and either `last_errored_at` is `None` or `now - last_errored_at >= 24h`, with the 24h window as a named constant beside it. `None` only happens for rows that were already `Errored` before the migration; treating them as due keeps today's behaviour for them. Both reconcilers replace their `status == Errored` filter with the predicate, so the threshold lives in one place.
- *Alternative:* measure from `updated_at`, with no migration. Rejected, because reconcile bumps `updated_at` on every pass, so the cooldown would never expire.
- *Alternative:* only bump `updated_at` when the title actually changes. Rejected: a renamed video would still restart its cooldown, the cooldown would be tied to an unrelated field, and it changes what `updated_at` means for every video.

**Order eligible tasks by `run_at ASC, id ASC`.** This changes only the `ORDER BY` in `list_eligible`. It relies on the same RFC 3339 lexical ordering the `run_at <= ?1` predicate already relies on (all values are written via `DateTime<Utc>::to_rfc3339`).
- *Alternative:* priority by task type (downloads before reconciles). That's more policy than needed, since due-time order alone stops old retries from jumping the queue.

## Risks / Trade-offs

- [Image size grows by roughly 100 MB (Deno binary)] → Acceptable for a NAS deployment. It's a single static layer.
- [A transient failure now waits up to about 24h (plus the next hourly pass) before reconcile retries it] → The per-task retries (5 attempts with growing backoff) still cover short outages. Only videos that exhaust those wait for the cooldown.
- [A deployment that still lacks Deno (old image) keeps failing, but only once a day] → Fixed by deploying the image from this change.
- [Rows already `Errored` before the migration have no `last_errored_at`, so they are retried on the first pass after deploy] → A one-off, and the same as today's behaviour. If they fail again they get a timestamp and follow the cooldown from then on.

## Migration Plan

1. Build and release the image, then pull and restart the container on the NAS. The new migration (nullable `videos.last_errored_at`) runs at startup.
2. Tasks already queued keep their retry schedule. With Deno present, their next attempt succeeds. Dead-lettered ones are picked up by the first reconcile pass at least 24h after they errored. No manual cleanup is required.
3. Rollback: redeploy the previous image. An older binary ignores the extra nullable column.
