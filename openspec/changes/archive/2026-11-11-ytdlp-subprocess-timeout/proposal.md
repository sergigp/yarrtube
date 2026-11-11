# Proposal

## Why

Every `yt-dlp` call runs through a blocking `Command::output()` with no deadline, no kill and no `--socket-timeout`. A `yt-dlp` (or its `ffmpeg`/`deno` child) that never exits holds its lane slot forever, and within an hour the self-rescheduling exclusive `UpdateYtdlp` task becomes due and drains the executor: no task in any lane starts again until the container is restarted, while the heartbeat still logs "alive" and the tasks UI shows "running".

## What Changes

- Every `yt-dlp` invocation runs with a deadline picked by its kind: **short** (15 min) for channel listings, quality/caption probes, chapter fetches, subtitle and thumbnail fetches and the diagnostic probe; **long** (4 h) for the media download. Fixed constants, not configurable (no evidence yet that any install needs other values; an env var can be added later without changing the behaviour).
- On expiry the whole process group is killed (`SIGKILL`), so `ffmpeg`/`deno` children die too, the child is reaped, and the call returns an error saying `yt-dlp` timed out after the deadline. It surfaces as a systemic error, so the normal retry path (failed attempt / task retry) takes over; a timed-out download is not diagnosed.
- stdout and stderr are drained concurrently while waiting, so a large output can't deadlock the pipe.
- The process group is also killed when `yt-dlp` exits in time, so a child it left behind can't hold the pipes open (an unbounded wait) or keep running after the call.
- The Docker image runs `tini` as PID 1, so killed `ffmpeg`/`deno` children, reparented to PID 1, are reaped instead of staying zombies under `yarrtube`.
- Every argument list passes `--socket-timeout 30`, so a stalled connection fails inside `yt-dlp` long before the deadline.
- Unchanged: the "text file busy" spawn retry, the exit-code → `Success` / `CleanFailure` classification, the not-found message.
- **Follow-up, not built here**: show a running task's elapsed time in the tasks UI, so a slow/hung task is visible.

## Capabilities

### New Capabilities
- `ytdlp-invocation`: how the daemon runs a `yt-dlp` process — bounded run time per invocation kind, killing the process tree on expiry, and the network timeout passed to every call.

### Modified Capabilities
(none — a timed-out call is already "yt-dlp failing to run", a failed attempt under `video-retries`)

## Impact

- `src/infrastructure/shared/ytdlp.rs`: `YtDlpCommand` gains a deadline, `output()` becomes spawn + reader threads + bounded wait + group kill; every public function picks its deadline and adds `--socket-timeout`.
- `Dockerfile`: `tini` installed and used as the entrypoint (`tini -- docker-entrypoint.sh`); `src/serve.rs`'s `shutdown_signal` comment no longer assumes `yarrtube` is PID 1.
- New direct dependency `libc` (already in `Cargo.lock` transitively) for `killpg`.
- yt-dlp now runs in its own process group: a Ctrl-C in a terminal running `scripts/run-local.sh` no longer reaches an in-flight `yt-dlp` directly (it exits when the daemon's runtime drops it or on container stop).
- No domain, application, HTTP or UI changes.
