## Why

The container never bakes a `yt-dlp` binary into the image and never persists
it across restarts — every container start depends on one unretried network
call to GitHub, at startup, to produce a usable binary. When that call fails
(observed on the NAS: a permission mismatch between the entrypoint's
`PUID`/`PGID` drop and the root-owned `/usr/local/bin`, but any transient
network failure has the same effect), the daemon starts "healthy" anyway with
no `yt-dlp` on `PATH` for its entire run, and every video queued during that
window burns through its 5 retry attempts in about 2 minutes before dead-
lettering permanently — with no code path that ever revisits a permanently
failed video. A single missing binary therefore turns into unbounded,
silent, manual-intervention-required downtime instead of a brief, self-
healing blip.

## What Changes

- The Docker image bundles a known-good `yt-dlp` binary at build time, so a
  freshly started container is never without a working one; the existing
  startup fetch becomes a best-effort upgrade over that floor rather than the
  only source.
- `update-ytdlp`'s install location and `download`'s invocation of `yt-dlp`
  are unified: `download` always invokes the binary at the currently
  configured path directly, rather than relying on a `PATH` lookup that
  happens to coincide with the install location today.
- The default install location moves from `/usr/local/bin` (root-owned,
  outside the entrypoint's `chown` scope) to a location under `/app` (already
  `chown`ed for `PUID`/`PGID`), removing the permission mismatch that caused
  the observed failure.
- The daemon gains a recurring, self-rescheduling hourly task that re-runs
  the `yt-dlp` update for as long as it runs — regardless of whether the
  previous attempt succeeded — so a failed startup update, or a binary that
  degrades later, self-heals without an operator running `docker exec
  update-ytdlp` or restarting the container.
- Task retry delay changes from a fixed 30-second gap to a delay that grows
  with the retry count, applied uniformly to every task type, so a
  `download_video` task's 5 attempts are spread over several hours instead of
  about 2 minutes — giving the hourly self-update loop realistic time to fix
  a systemic `yt-dlp` problem before a video's attempts are exhausted.
- A video whose download task exhausts its retries (`Errored`) is no longer a
  dead end: the recurring playlist reconcile pass now also resets `Errored`
  videos to `Pending` and reschedules a download, the same way it already
  heals a `Downloaded` video whose file went missing. This runs
  unconditionally on every reconcile pass, with no cap on how many times a
  given video may be retried this way.

## Capabilities

### New Capabilities

(none — this change extends existing capabilities)

### Modified Capabilities

- `container-image`: the image build now bundles a `yt-dlp` binary as part of
  what makes the image runnable, alongside the existing muxing tool.
- `ytdlp-self-update`: the binary `download` invokes and the binary
  `update-ytdlp` installs are now the same configured path, invoked directly
  rather than via a `PATH` lookup; automatic invocation now also happens on a
  recurring interval while the daemon runs, not only once at startup.
- `task-scheduling`: the fixed retry delay is replaced by a delay that grows
  with the task's retry count, still giving up after 5 attempts.
- `playlist-reconciliation`: the recurring reconcile pass now also recovers
  videos whose download permanently failed, not only videos already marked
  `Downloaded` whose file went missing.
- `video-download`: a video that exhausts its download retries is no longer
  guaranteed to never be attempted again on its own — the
  `playlist-reconciliation` capability may revive it on a later reconcile
  pass.

## Impact

- `Dockerfile`: bundle a `yt-dlp` Linux binary into the image at build time.
- `src/infrastructure/shared/ytdlp.rs`: invoke `yt-dlp` via the configured
  path instead of a bare `Command::new("yt-dlp")` `PATH` lookup.
- `src/cli/ytdlp_update.rs`: change the default `YTDLP_PATH` to a location
  under `/app`.
- `src/domain/task/scheduled_task.rs`: replace the fixed `RETRY_DELAY_SECONDS`
  with a retry-count-based delay formula.
- `src/tasks/`, `src/serve.rs`: add a recurring, self-rescheduling `yt-dlp`
  self-update task and register it alongside the existing task handlers.
- `src/domain/video/service.rs` (`reconcile_filesystem`): also reset
  `Errored` videos to `Pending` and reschedule their download.
- `docker-entrypoint.sh`: no functional change expected (the new install
  location already falls under its existing `/app` `chown`), but worth
  re-checking once the path changes.
- `README.md`: update the documented `YTDLP_PATH` default and the
  "recreated on restart" note to reflect the bundled-binary floor.
- OpenSpec: delta specs for `container-image`, `ytdlp-self-update`,
  `task-scheduling`, `playlist-reconciliation`, `video-download`.
