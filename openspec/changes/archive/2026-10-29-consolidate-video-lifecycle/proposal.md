# Proposal

## Why

A stored video's lifecycle is split between `VideoReconciler` (step scheduling, categories on sync) and the playlist/channel reconcile pass (broken-file reset, errored recovery, metadata repair, stranded detection, bulk category repair), with special cases (`schedule_download`, the `handled` set) stitching the two together. Making the per-video sync the single owner of a video's desired state keeps every lifecycle rule in one idempotent, self-healing place, so future per-video work (e.g. subtitle post-processing) is added there instead of across passes and subscribers.

## What Changes

- The video sync becomes the only place that brings one stored video to its desired state: it assigns its source's categories, resets a Downloaded video whose file is missing or not mp4, recovers an Errored video after its 24h cooldown, repairs missing metadata (legacy relocation unchanged), warns about a stranded video, and schedules the still-needed steps.
- A reset video is treated like any other Pending video: its sync schedules both its download and its thumbnail fetch (previously only the download).
- The playlist/channel reconcile pass only reconciles the stored membership with YouTube, runs the video sync inline for every stored video (including those it just added), then sweeps orphaned files.
- The pass no longer assigns categories in bulk; each video sync does. A pass that repairs several videos' categories publishes one video-categories-changed event per repaired video instead of one per pass.
- Concurrent syncs of the same video (pass and `SyncVideo` task) are serialized by an in-memory per-video lock.
- Downloads, thumbnail and subtitle fetches stay separate tasks.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `video-sync`: the sync gains broken-file reset, errored recovery, missing-metadata repair and stranded detection; reset videos get the full sync decision; passes sync every stored video including newly added ones; concurrent syncs of one video are serialized.
- `playlist-reconciliation`: the per-video recovery requirements (missing file reset, permanently failed recovery, missing metadata/thumbnail/subtitles, stranded) move to `video-sync`; the pass keeps membership, orphan sweep, and syncing every stored video.
- `channel-video-sync`: same as `playlist-reconciliation` for channels.
- `video-subtitles`: cross-reference for pass-scheduled subtitle fetches now points to `video-sync`.
- `video-categories`: reconcile-pass category repair happens through each video's sync; change notification is one event per video sync, not per pass.

## Impact

- `src/domain/services/video_reconciler.rs`: owns the full lifecycle; trait reduced to `reconcile(id)`; new dependencies (video file, metadata, relocation).
- `src/domain/services/internal_video_reconciler.rs`: reduced to orphan cleanup plus syncing every stored video; `MembershipDelta`, `handled`, `sort_positions`, `schedule_download` removed.
- `src/domain/services/channel_video_reconciler.rs`, `playlist_video_reconciler.rs`: no `assign_source`, no delta passed down.
- `src/domain/services/video_category_assigner.rs`: `assign_source` removed.
- `doc/ARCHITECTURE.md`: step 6 of the channel flow.
- `src/serve.rs`: wiring of the new `VideoReconciler` dependencies.
- No API, database or UI change.
