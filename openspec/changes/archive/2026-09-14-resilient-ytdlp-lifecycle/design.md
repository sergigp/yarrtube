## Context

See `proposal.md` - Why for the motivating incident. The relevant current-state facts that shape this design:

- `Dockerfile` installs no `yt-dlp` binary; the image only has `ffmpeg`, `ca-certificates`, `gosu`. `README.md` states the `yt-dlp` binary is "recreated on restart" by design — nothing persists it.
- `serve::run()` calls `run_startup_ytdlp_update()` once, synchronously, before the tokio runtime starts; failure is logged and swallowed, startup proceeds regardless.
- `docker-entrypoint.sh` `chown -R $PUID:$PGID` on `/app` and `/videos` only. The default `YTDLP_PATH` (`/usr/local/bin/yt-dlp`) is outside that scope and stays root-owned when `PUID`/`PGID` are set — the confirmed cause of the observed `Failed to create temp file` error.
- `infrastructure/shared/ytdlp.rs::download_video` invokes `Command::new("yt-dlp")`, a bare `PATH` lookup, independent of `cli/ytdlp_update.rs::target_path()` (`YTDLP_PATH`). The two only agree today because the default install path happens to sit on `PATH`.
- `domain/task/scheduled_task.rs` has one shared `MAX_ATTEMPTS = 5` / `RETRY_DELAY_SECONDS = 30` pair used by every task type (`download_video`, `reconcile_playlist`, `delete_video_file`); `ScheduledTask::fail` has no visibility into *why* a task failed, only that it did.
- `domain/video/service.rs::reconcile_filesystem` only revisits videos already `Downloaded`; a video that reaches `Errored` (task dead-lettered) has no code path back to `Pending` — confirmed by reading `sync_playlist_membership` (only adds/removes, never resets status) and `src/http/` (no retry endpoint).
- `Video` carries no stored failure reason — the error string only ever reaches `ScheduledTask.last_error` and the dead-letter table, disconnected from the `Video` row.

## Goals / Non-Goals

**Goals:**
- A freshly started container is never without a usable `yt-dlp` binary, regardless of network conditions or the `update-ytdlp` fetch's success.
- A systemic `yt-dlp` failure self-heals without an operator running `docker exec ... update-ytdlp` or restarting the container.
- A `download_video` task's retry window is wide enough (hours, not ~2 minutes) that the hourly self-heal loop realistically gets a chance to fix a systemic problem before that task's own attempts are exhausted.
- A video that does exhaust its attempts and reaches `Errored` is not permanently stuck — it gets another chance on every future reconcile pass, indefinitely.

**Non-Goals:**
- Distinguishing a systemic failure (e.g. `yt-dlp` missing) from a per-video failure (e.g. `yt-dlp` ran and failed for this video) at the task-retry level. Both are treated identically and consume the same retry budget — considered and deliberately dropped in favor of the simpler combination of wider backoff + the hourly self-heal task + uncapped reconcile-driven recovery.
- A cap on how many times reconcile may recover a permanently-errored video. A genuinely dead video (deleted, region-locked) will be retried on every future reconcile pass, forever; this is accepted as an acceptable cost, not solved here.
- Per-task-type retry configuration. The backoff formula is one shared curve applied uniformly to every task type (`download_video`, `reconcile_playlist`, `delete_video_file` alike), not configurable per type.
- Changing `VideoStatus`'s vocabulary. `ErroredRetrying` and `Errored` are reused as-is; `Errored` simply stops meaning "will never be attempted again."

## Decisions

### Bundle `yt-dlp` into the image at build time
The `Dockerfile`'s `build` or `runtime` stage fetches the latest stable Linux `yt-dlp` binary the same way `update-ytdlp` does (same GitHub releases API, same asset name) and installs it at the configured path, so the image itself is never missing one. The runtime startup/recurring update becomes a pure upgrade over this floor rather than the sole source.
- **Alternative considered**: keep relying solely on the runtime fetch (status quo). Rejected — it's the root architectural cause of unbounded downtime: one failed network call away from zero functionality for the container's entire lifetime.
- **Alternative considered**: persist `/usr/local/bin` (or wherever `yt-dlp` lives) in a volume so a successful install survives restarts. Rejected for now — doesn't help a container's *first* start (fresh volume), adds a volume/permissions surface, and the bundle-at-build-time approach already gives every start a working floor without it.

### Move the default `YTDLP_PATH` under `/app`, and invoke via the configured path directly
Default install location moves from `/usr/local/bin/yt-dlp` to a location under `/app` (e.g. `/app/bin/yt-dlp`), which `docker-entrypoint.sh` already `chown -R $PUID:$PGID`s — no entrypoint change needed. `download_video` (`infrastructure/shared/ytdlp.rs`) stops calling `Command::new("yt-dlp")` and instead invokes the binary at the same configured path (`YTDLP_PATH` / `target_path()`) that `update-ytdlp` installs to, removing the implicit, coincidental coupling through `PATH`.
- **Alternative considered**: leave the path at `/usr/local/bin` and have `docker-entrypoint.sh` additionally `chown` it. Rejected — still leaves `download_video`'s bare `PATH` lookup as an implicit, undocumented dependency on the install location; moving under `/app` and invoking explicitly closes both the permission bug and that coupling in one move, per the earlier discussion in this change's design conversation.

### Recurring self-update task, not a bespoke background loop
A new task type (e.g. `update_ytdlp`) follows the same self-rescheduling pattern `reconcile_playlist` already uses: run the update, then unconditionally schedule the next occurrence an hour out — regardless of whether this attempt succeeded. Registered in `tasks::registry()` like any other handler, executed by the existing `TaskExecutor` poll loop. Scheduled once at startup (in addition to, not instead of, the existing immediate startup attempt) to seed the recurring chain.
- **Alternative considered**: a fourth bespoke `tokio::spawn` loop alongside `heartbeat_loop`. Rejected — the task system already provides persisted, restart-safe recurring scheduling; reusing it avoids a parallel, unpersisted mechanism that would forget its schedule across restarts and duplicates `reconcile_playlist`'s existing shape.
- This task must never dead-letter: unlike `download_video`, a failed attempt is not a "this specific unit of work failed" case — it should simply try again next hour, forever. It reschedules itself directly (like `reconcile_playlist`) rather than going through the generic fail-and-retry machinery in `ScheduledTask::fail`, so it is unaffected by the retry-count-based delay below and by `MAX_ATTEMPTS`.

### Retry-count-based delay, applied uniformly
`ScheduledTask::fail` computes `run_at` as a function of the post-increment `retries` count (e.g. `base_seconds * multiplier.pow(retries)`) instead of the flat `RETRY_DELAY_SECONDS`. `MAX_ATTEMPTS` stays 5. Concrete constants (base/multiplier) are chosen so `download_video`'s 5 attempts span roughly five hours (e.g. ~10min, ~30min, ~1.5h, ~3h between successive attempts) — final numbers are an implementation-time tuning detail, not a spec-level requirement.
- Applied to the one shared `ScheduledTask::fail`, so every task type retries on the same growing curve. Explicitly not made per-task-type-configurable (see Non-Goals) to keep the change small; `reconcile_playlist` and `delete_video_file` retrying more slowly on failure is an accepted trade-off.

### Reconcile also recovers `Errored` videos
`reconcile_filesystem` (or a sibling step in the same reconcile pass) is extended: alongside its existing loop over `Downloaded` videos (healing missing/wrong-format files), it also finds videos with status `Errored`, resets them to `Pending` (via the existing `reset_for_redownload`), and schedules a `DownloadVideo` task — the same recovery shape already used for a `Downloaded` video whose file went missing.
- **Alternative considered**: a reactive approach — detect "yt-dlp just came back healthy" and requeue only videos that failed for that specific reason. Rejected (see Non-Goals) — requires storing a failure reason on `Video` (new field/migration) and new event/subscriber wiring; the blind, unconditional reconcile-driven sweep is simpler, reuses the existing hourly-by-default reconcile cadence, and was judged sufficient.
- No cap on recovery attempts. A permanently broken video (deleted, private, region-locked) will be reset and re-attempted on every future reconcile pass, indefinitely. Accepted cost, not solved here.

## Risks / Trade-offs

- **[Risk]** Reconcile-driven recovery retries a genuinely permanently-broken video forever, generating a failed `download_video` task (and its own 5-attempt/hours-long backoff sequence) on every reconcile cycle. → **Mitigation**: none built into this change; accepted as a low-cost trade-off (an occasional failed `yt-dlp` invocation) for not needing a failure-reason field or a retry cap. Revisit if it proves noisy in practice.
- **[Risk]** Slower retries mean a video that fails for a real, quickly-transient reason (e.g. a momentary network blip unrelated to `yt-dlp` itself) now waits up to hours between its own retries instead of 30 seconds, and up to ~5h to dead-letter — plus, once it does dead-letter, up to one reconcile interval (default 1h) before being retried again. → **Mitigation**: the on-demand reconcile HTTP endpoint (existing `playlist-reconciliation` capability) already lets an operator force an immediate recovery pass rather than waiting.
- **[Risk]** Applying the slower backoff to every task type, not just `download_video`, means `reconcile_playlist` and `delete_video_file` also retry more slowly on failure. → **Mitigation**: accepted (see Non-Goals); these task types fail far less often in practice than `download_video` depending on an external binary.
- **[Risk]** The image-bundled `yt-dlp` binary will drift out of date over the life of a long-lived container between successful runtime updates. → **Mitigation**: this is exactly what the recurring hourly self-update task addresses; the bundled binary is only ever a floor, not a substitute for staying current.

## Migration Plan

- No data migration: no schema changes (no new `Video` fields, no new tables) — `Errored`/`ErroredRetrying` are reused as-is, and the new recurring task uses the existing `tasks`/dead-letter tables.
- Rollout is a normal image rebuild + redeploy: the new build step adds the bundled binary, the entrypoint needs no change (new path is already under `/app`'s existing `chown`), and existing `Errored` videos from before this change will simply get picked up by the next reconcile pass after deploying, with no manual backfill needed.
- Rollback is a normal image rollback; no forward-only state is introduced.
