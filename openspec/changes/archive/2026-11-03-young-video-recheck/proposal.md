## Why

Some per-video YouTube data isn't there yet right after upload (automatic captions, YouTube's auto chapters, HD formats still processing), but is final a few days later. Each data type handles this differently today:
- Subtitles: years-old videos are re-probed for 7 days after their first empty check. On the NAS that's ~540 `fetch_subtitles` tasks for Catalan kids' playlists. Each is a ~25s yt-dlp extraction in a one-at-a-time lane, so draining them takes longer than the 3h reconcile interval and the queue never empties.
- Chapters: checked once and stored forever, even when a fresh upload has none yet.
- Download quality: whatever a fresh upload offered is kept, even an SD fallback.
- Failed or inconclusive checks: retried every reconcile pass, with no end.

One shared policy fixes all of these.

## What Changes

- New shared policy for late YouTube data:
  - A video is **young** until 7 days after its publish date (the record's creation time when the publish time is unknown).
  - While young, an empty or below-quality result is provisional and re-checked at most once every 24h.
  - Once old, the result is final and never checked again.
  - A failed or inconclusive check is retried at most once every 24h. On an old video it is given up after 3 failed attempts.
- **Subtitles**:
  - An old video whose check finds no captions becomes Unavailable on that first check.
  - The "first check without captions" window is replaced by the shared policy.
  - Inconclusive probes (warnings) and probe failures now count towards the 3-attempt limit.
  - **BREAKING** (data): the recorded "first check without captions" time is replaced by "last checked" and "failed attempts".
- **Chapters**:
  - A young video found without chapters is re-checked daily until it's old.
  - An old video's result is final.
  - A failed chapters fetch counts towards the same 3-attempt limit.
- **Download quality**:
  - Each download records the video height it actually got.
  - While a downloaded video is young and below its tier's target, a daily quality check asks YouTube which height its tier would get now. If that's higher, the video is downloaded again and its file replaced in place, keeping its episode, subtitles, chapters and watch state.
  - Each tier has a settlement height: high 1080, mid 720, low 480. Reaching it ends the checks; high downloads themselves stay uncapped.
  - Once old, the quality is accepted.
- Video sync schedules subtitle, chapters and quality checks only while their data is unsettled and the last check is at least 24h old. Reconcile passes stop re-queueing the same probes every 3h.
- Migration rules, so the existing library isn't re-probed or re-downloaded:
  - Existing checked chapters and downloaded quality are treated as settled.
  - Existing Missing subtitles keep a last-checked time but have no failed attempts.

## Capabilities

### New Capabilities
- `late-video-data`: the shared young/old policy (7-day youth, 24h re-check spacing, 3-attempt failure limit) that every piece of late YouTube data follows.

### Modified Capabilities
- `video-subtitles`: Subtitle State and Subtitle Fetch Outcome follow the shared policy instead of the 7-day first-missing window, and failures are bounded.
- `video-chapters`: chapters found empty on a young video are re-checked, and failed fetches are bounded.
- `video-download`: the downloaded height is recorded, and a young video below its tier's target is upgraded in place.
- `video-sync`: subtitle and chapters fetches are scheduled only when unsettled and due, and a quality check step is added.
- `task-listing`: the new quality check task carries its video's context.
- `web-ui`: the tasks view describes quality check tasks.

## Impact

- Domain:
  - `Video` gets per-data check bookkeeping (last checked, failed attempts, settled) and a downloaded height.
  - New shared policy type.
  - `VideoSyncPlan` decides with the current time.
- Services:
  - `SubtitleFetcher` and `ChapterFetcher` change.
  - `VideoDownloader` records the height.
  - New `QualityUpgrader` and `upgrade_quality` task in the Download lane.
- yt-dlp adapter:
  - Download prints the height.
  - Chapters fetch also prints the publish time.
  - New quality probe.
- SQLite: migration adding columns, backfilling settled flags and renaming `subtitles_first_missing_at`.
- Task listing (backend and web UI) gains the `upgrade_quality` task.
- After deploy, the NAS subtitle backlog clears on the next pass: those videos are old, so they become Unavailable on their next check.
