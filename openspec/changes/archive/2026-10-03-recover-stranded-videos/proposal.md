## Why

A video can end up stranded: still in a non-terminal status (Pending, In Progress or Errored Retrying) with no download task left to settle it. Reconcile recovery only picks up Errored videos, so nothing ever downloads a stranded video again. This happens when:

- the final attempt's status write fails, so the task is dead-lettered and the video stays Errored Retrying;
- crash recovery dead-letters a task that was running its last attempt without calling the handler, so the video stays In Progress or Errored Retrying.

In prod there are 6 stranded videos right now: 2 Errored Retrying, 3 Pending (since 09-28/09-30) and 1 In Progress (since 10-01). Two of them are the copyright-claimed Yakari videos. The diagnostic probe now reports "claimed content" for them, so a single new attempt would exclude them, but no attempt ever runs. They also get a pointless thumbnail fetch on every reconcile.

## What Changes

- Each playlist and channel reconcile pass reschedules a download for every stored video that is stranded: status Pending, In Progress or Errored Retrying, and no download task pending or running for it. The video's status is left unchanged, so the download moves it on as usual. There is no cooldown.
- A video the same pass newly adds or resets for redownload is not treated as stranded. Its download is already being scheduled.
- A download task whose video has already settled (Downloaded, Excluded or Errored) by the time it runs is skipped without touching the video. This guards against a reconcile that read a stale status and rescheduled a video whose download had just finished.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `playlist-reconciliation`: adds the Stranded Video Recovery requirement.
- `channel-video-sync`: adds Stranded Video Recovery (Channels).
- `video-download`: adds Download Skipped For an Already-Settled Video.

## Impact

- `src/domain/services/playlist_video_reconciler.rs`, `src/domain/services/channel_video_reconciler.rs`: the stranded-video pass.
- `src/domain/services/video_downloader.rs`: the settled-video guard.
- `src/domain/video/video.rs`, `src/domain/task/scheduled_task.rs`: small helpers.
- No schema, API or UI change. On the first pass after deploy, the 6 prod videos heal: the 2 Yakari ones become Excluded and the rest download or follow the normal retry flow.
