# Spec Delta

## Purpose

Bounds how long the daemon lets one `yt-dlp` process run, so a hung `yt-dlp` (or a child it started) can never hold a task slot, and through the exclusive update, the whole task queue, forever.

## ADDED Requirements

### Requirement: Bounded yt-dlp Run Time
Every `yt-dlp` invocation SHALL have a deadline: 4 hours for a media download, 15 minutes for every other invocation (channel listings, quality and caption probes, chapter, subtitle and thumbnail fetches, the diagnostic probe). An invocation that exits before its deadline SHALL return its result as before.

#### Scenario: Download within its deadline
- **WHEN** a media download's `yt-dlp` exits after 2 hours
- **THEN** its result is handled as today (success or clean failure by exit code)

#### Scenario: Probe past its deadline
- **WHEN** a quality probe's `yt-dlp` is still running 15 minutes after it started
- **THEN** the probe is stopped and fails

### Requirement: Expired yt-dlp Is Killed With Its Children
When an invocation reaches its deadline, the system SHALL kill the `yt-dlp` process and every process it started (e.g. `ffmpeg`, `deno`), and SHALL wait for `yt-dlp` to be reaped before returning, so no process of that invocation outlives the call.

#### Scenario: Hung download with a running ffmpeg
- **WHEN** a download reaches its deadline while `yt-dlp` is waiting on an `ffmpeg` child
- **THEN** both `yt-dlp` and `ffmpeg` are gone when the call returns

### Requirement: No Leftover Process After A Finished yt-dlp
When `yt-dlp` exits before its deadline, the system SHALL kill every process it started that is still running, so none can hold its output open and keep the call waiting, or keep running after the call.

#### Scenario: yt-dlp crashes and leaves its ffmpeg running
- **WHEN** `yt-dlp` exits while an `ffmpeg` child it started is still running with its output
- **THEN** the call returns `yt-dlp`'s result at once and the `ffmpeg` child is gone

### Requirement: Killed Processes Are Reaped In The Image
The Docker image SHALL run an init process as PID 1 that reaps orphaned processes, so the killed children of a `yt-dlp` invocation never stay behind as zombies.

#### Scenario: Timed-out download in the container
- **WHEN** a download times out in the container and its `ffmpeg` child is killed
- **THEN** no zombie `ffmpeg` remains in the container's process table

### Requirement: Timed-Out Invocation Fails As A Systemic Error
A timed-out invocation SHALL fail with an error stating that `yt-dlp` timed out and after how long, and SHALL NOT be treated as a clean `yt-dlp` failure: a timed-out download is not diagnosed and never marks a video Unavailable. The calling step SHALL handle it like `yt-dlp` failing to run (a failed attempt, or a task failure for the task queue to retry), freeing its task slot.

#### Scenario: Download times out
- **WHEN** a video's download times out
- **THEN** the download is Retrying with one more failed attempt whose reason says `yt-dlp` timed out after 4 hours, and its task slot is free for other tasks

#### Scenario: Channel listing times out
- **WHEN** a channel reconcile's listing times out
- **THEN** the reconcile task fails with the timeout error and is retried by the task queue

#### Scenario: Update no longer blocked
- **WHEN** a download hangs while the yt-dlp update task is due
- **THEN** the hung download is killed at its deadline, the update runs once nothing else is running, and other tasks start again after it

### Requirement: Network Timeout On Every Call
Every `yt-dlp` invocation SHALL pass `--socket-timeout 30`, so a stalled connection fails inside `yt-dlp` instead of waiting for the run-time deadline.

#### Scenario: Any invocation
- **WHEN** the daemon runs `yt-dlp` for a download, listing, probe or fetch
- **THEN** its arguments include `--socket-timeout 30`
