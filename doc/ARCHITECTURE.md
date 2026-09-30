# Architecture

Yarrtube is a single process (`yarrtube serve`) backed by one SQLite database.
Inside it, the HTTP API does only the synchronous part of each request: it
validates the input, saves state and publishes a **domain event**. Everything
slow or fallible (talking to YouTube, running `yt-dlp`, touching the
filesystem) happens later in the background, driven by two SQLite-backed
queues:

- **Domain events** (`events` table): facts about something that already
  happened, such as `channel_created` or `video_added_to_channel`. Each event
  type can have any number of **subscribers**.
- **Tasks** (`tasks` table): units of work to run at a given time, such as
  `reconcile_channel` or `download_video`. Each task type has exactly one
  **handler**.

Two background loops poll these tables every 5 seconds: `DomainEventsConsumer`
for events, one event at a time, and `TaskExecutor` for tasks, several at a
time in **lanes** (see [Task lanes](#task-lanes)). When something keeps
failing, it ends up in a **dead letter** table (`domain_events_dead_letter` /
`tasks_dead_letter`) for inspection instead of being retried forever.

## Overview

```mermaid
flowchart TB
    client(["Web UI / HTTP client"])
    api["HTTP API (axum)"]
    services["Domain services<br/>ChannelCreator, ChannelVideoReconciler,<br/>VideoDownloader, ..."]
    external(["YouTube API / yt-dlp / filesystem"])

    subgraph db["SQLite"]
        direction LR
        events[("events")]
        tasks[("tasks")]
    end

    subgraph events_side["Domain events"]
        consumer["DomainEventsConsumer<br/>polls every 5s"]
        subscribers["Subscribers<br/>(N per event type)"]
        events_dl[("domain_events_dead_letter")]
    end

    subgraph tasks_side["Tasks"]
        executor["TaskExecutor<br/>polls every 5s,<br/>runs tasks in lanes"]
        handlers["Task handlers<br/>(1 per task type)"]
        tasks_dl[("tasks_dead_letter")]
    end

    client --> api --> services
    services --> external
    services -- "publish event" --> events
    services -- "schedule task" --> tasks

    events --> consumer -- "dispatch" --> subscribers
    consumer -. "5th failure" .-> events_dl
    tasks --> executor -- "dispatch" --> handlers
    executor -. "5th failure" .-> tasks_dl

    subscribers -- "call service /<br/>schedule task" --> services
    handlers -- "call service" --> services
```

Subscribers and handlers are thin adapters: they decode a JSON payload and call
a domain service. Domain services can publish new events and schedule new
tasks, so one user action fans out into a chain of background work.

## Retries and dead letters

Both queues give an item 5 attempts. The difference is when the next attempt
runs:

| | Domain events | Tasks |
|---|---|---|
| Dispatch | Every subscriber registered for the type | The single handler for the type |
| On failure | Retried on the next poll (no delay) | Retried after `base * 3^retries` seconds (base `YARRTUBE_RETRY_BASE_DELAY_SECONDS`, default 150) |
| After 5 failed attempts | Moved to `domain_events_dead_letter` | Moved to `tasks_dead_letter` |
| On success | Row deleted | Row deleted |

An event is handled as a whole. If one of its subscribers fails, the whole
event is retried, so subscribers must be idempotent.

A task that was left `running` by a crash is recovered at startup and counted
as a failed attempt. A task whose handler panics is counted as a failed
attempt too.

```mermaid
stateDiagram-v2
    direction LR
    [*] --> Pending: published / scheduled
    Pending --> Pending: poll, handler fails,<br/>attempts left (retry)
    Pending --> Done: poll, handler succeeds
    Pending --> DeadLetter: poll, 5th failure
    Done --> [*]: row deleted
    DeadLetter --> [*]: kept in *_dead_letter
```

## Task lanes

The `TaskExecutor` runs tasks in parallel, grouped into lanes. Each lane has
its own cap on how many of its tasks run at once, so slow work in one lane
never holds up another.

| Lane | Task types | Runs at once |
|---|---|---|
| Download | `download_video` | `YARRTUBE_DOWNLOAD_CONCURRENCY` (default 2) |
| Thumbnail | `fetch_thumbnail` | 1 |
| Light | reconciles and file deletions | 1 |
| Exclusive | `update_ytdlp` | 1, and nothing else |

On every poll, and right after any task finishes, the executor walks the
eligible tasks in `run_at` order and starts each one whose lane has room.
Within a lane, tasks start in `run_at` order. Lanes are not ordered against
each other. A few more rules apply:

- **Atomic claim.** A task is flipped from `pending` to `running` in a single
  statement before it starts, so it is never dispatched twice.
- **One task per video at a time.** A `download_video` and a
  `fetch_thumbnail` for the same video never run together. A task held back
  this way is skipped, and the tasks behind it in its lane still start.
- **No duplicate video tasks.** Scheduling a `download_video` or
  `fetch_thumbnail` for a video that already has one `pending` or `running`
  adds nothing.
- **Exclusive update.** When `update_ytdlp` is due, no new task starts until
  everything running has finished. Then the update runs alone.

Which lane a task type belongs to, and which video it locks, are facts of the
domain (`Task::lane_for`, `Task::exclusivity_key`). The executor itself knows
nothing about specific task types.

## Example flow: track a channel and download its videos

1. `POST /api/channels` saves the channel and publishes `ChannelCreated`.
2. The `ReconcileOnChannelCreated` subscriber runs the first sync.
   - The sync reads the channel's most recent uploads from YouTube.
   - It saves each new video as `PENDING` and publishes one
     `VideoAddedToChannel` per video. It does not run `yt-dlp` itself, so a
     large channel or playlist is saved in seconds.
   - It schedules the next `ReconcileChannel` task one reconcile interval
     later (`YARRTUBE_RECONCILE_INTERVAL_SECONDS`, default 1 hour).
3. For each `VideoAddedToChannel`, two subscribers schedule a task each:
   - `FetchThumbnailOnVideoAddedToChannel` schedules a `FetchThumbnail`
     task,
   - `DownloadVideoOnVideoAddedToChannel` schedules a `DownloadVideo` task.
4. The `TaskExecutor` runs them in their lanes: thumbnails one at a time,
   downloads a few at a time. Both call `yt-dlp`. The thumbnail usually
   arrives first, so the UI can show it while the video is still downloading.
5. Every hour, the `ReconcileChannel` task runs the sync again and schedules
   the next one. New uploads repeat steps 3–4. The sync also schedules a
   `FetchThumbnail` for any video still missing its thumbnail.

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant API as HTTP API
    participant Events as events table
    participant Consumer as DomainEventsConsumer
    participant Reconciler as ChannelVideoReconciler
    participant Tasks as tasks table
    participant Executor as TaskExecutor
    participant YT as YouTube / yt-dlp

    User->>API: POST /api/channels
    API->>API: ChannelCreator saves channel
    API->>Events: publish ChannelCreated
    API-->>User: 201 Created

    Consumer->>Events: poll
    Events-->>Consumer: ChannelCreated
    Consumer->>Reconciler: ReconcileOnChannelCreated
    Reconciler->>YT: list most recent uploads
    YT-->>Reconciler: videos
    loop each new video
        Reconciler->>Reconciler: save video as PENDING
        Reconciler->>Events: publish VideoAddedToChannel
    end
    Reconciler->>Tasks: schedule ReconcileChannel (now + interval)
    Consumer->>Events: delete ChannelCreated

    Consumer->>Events: poll
    Events-->>Consumer: VideoAddedToChannel (xN)
    Consumer->>Tasks: FetchThumbnailOnVideoAddedToChannel<br/>schedules FetchThumbnail (run now)
    Consumer->>Tasks: DownloadVideoOnVideoAddedToChannel<br/>schedules DownloadVideo (run now)

    Executor->>Tasks: poll, claim what fits each lane
    par Thumbnail lane, one at a time
        Tasks-->>Executor: FetchThumbnail
        Executor->>YT: ThumbnailFetcher runs yt-dlp
        Executor->>Tasks: delete task (thumbnail recorded)
    and Download lane, up to YARRTUBE_DOWNLOAD_CONCURRENCY
        Tasks-->>Executor: DownloadVideo
    end
    Executor->>YT: VideoDownloader runs yt-dlp
    alt success
        YT-->>Executor: file written
        Executor->>Tasks: delete task (video DOWNLOADED)
    else failure, attempts left
        Executor->>Tasks: reschedule with backoff (video ERRORED_RETRYING)
    else 5th failure
        Executor->>Tasks: move to tasks_dead_letter (video ERRORED)
    end

    Note over Executor,Reconciler: One interval later, the ReconcileChannel task<br/>runs the same sync and schedules the next one.
```

Playlists follow the same shape with their own events (`PlaylistCreated`,
`VideoAddedToPlaylist`, ...) and tasks (`ReconcilePlaylist`). Deletions work
the same way: `ChannelDeleted` and `VideoRemovedFromChannel` schedule the
tasks that delete files.
