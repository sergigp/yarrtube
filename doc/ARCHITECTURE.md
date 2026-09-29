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
for events and `TaskExecutor` for tasks. When something keeps failing, it
ends up in a **dead letter** table (`domain_events_dead_letter` /
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
        executor["TaskExecutor<br/>polls every 5s"]
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
as a failed attempt.

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

## Example flow: track a channel and download its videos

1. `POST /api/channels` saves the channel and publishes `ChannelCreated`.
2. The `ReconcileOnChannelCreated` subscriber runs the first sync.
   - The sync reads the channel's most recent uploads from YouTube.
   - It saves each new video as `PENDING` and publishes one
     `VideoAddedToChannel` per video.
   - It schedules the next `ReconcileChannel` task one reconcile interval
     later (`YARRTUBE_RECONCILE_INTERVAL_SECONDS`, default 1 hour).
3. For each `VideoAddedToChannel`, the
   `DownloadVideoOnVideoAddedToChannel` subscriber schedules a `DownloadVideo`
   task.
4. The `TaskExecutor` runs each `DownloadVideo` task, which calls `yt-dlp`.
5. Every hour, the `ReconcileChannel` task runs the sync again and schedules
   the next one. New uploads repeat steps 3–4.

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
    Consumer->>Tasks: DownloadVideoOnVideoAddedToChannel<br/>schedules DownloadVideo (run now)

    Executor->>Tasks: poll
    Tasks-->>Executor: DownloadVideo
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
