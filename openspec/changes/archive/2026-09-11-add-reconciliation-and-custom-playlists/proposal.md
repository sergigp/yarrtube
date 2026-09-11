## Why

Playlist maintenance today is a one-shot sync: it assumes YouTube is the only
source of truth, and it only ever reacts to a single event once — it never
notices or heals drift between what the database says should exist and what
actually exists on disk (a crash between writing a file and recording it, a
file deleted or removed by hand, a video removed from its playlist while its
download is still in flight and the resulting file orphaned). It also can't
support a playlist that isn't backed by a YouTube playlist at all, which
blocks a planned browser-extension workflow for saving individual videos into
playlists that exist only in this system. This change reframes playlist
maintenance as reconciliation — a recurring, self-healing loop that converges
reality onto declared state, in the spirit of a Kubernetes controller — and
proves that model against a second kind of playlist that has no YouTube
playlist behind it, so the design doesn't stay YouTube-specific by accident.

## What Changes

- Rename the recurring per-playlist job from "sync" to "reconcile"
  (`SyncPlaylistTask` → `ReconcilePlaylistTask`) and broaden what it does each
  run: for a YouTube-linked playlist, diff membership against YouTube exactly
  as today (add newly-seen videos as `PENDING`, delete stored videos no
  longer on YouTube); for every playlist regardless of kind, also reconcile
  the filesystem against the database — heal a `Downloaded` video whose file
  is missing (reset it and redownload) and delete a file on disk that no
  longer corresponds to any `Downloaded` video (an orphan, e.g. one left
  behind when a video is removed while its download is still running).
- Record the actual on-disk filename on a video once its download succeeds,
  and use that exact filename for cleanup and for the new reconciliation
  pass, replacing today's sanitized-title-plus-`[video_id]`-suffix guessing.
- Add a `kind` to playlists: `YoutubeLinked` (today's behavior — an existing
  YouTube playlist ID, validated against the YouTube API, membership only
  ever changes via reconciliation, no manual add/remove) or `Custom` (a
  caller-supplied UUID with no YouTube playlist behind it, never reconciled
  for membership, videos added or removed only through new endpoints).
- Add `POST /custom-playlists` (create a custom playlist), `POST
  /custom-playlists/{id}/videos` (add a video by YouTube URL or ID, verified
  and titled via a new single-video YouTube API lookup), and `DELETE
  /custom-playlists/{id}/videos/{video_id}` (remove a video) — all reusing
  the existing `VideoAdded`/`VideoDeleted` domain events and the unchanged
  download/cleanup pipeline underneath.
- `GET /playlists` and `DELETE /playlists/{id}` stay unified across both
  kinds; only creation and video-membership mutation differ by kind.
- No manual removal is added for a YouTube-linked playlist's videos —
  YouTube stays authoritative for that kind, so removing a video there still
  means editing the YouTube playlist itself.
- **BREAKING**: the persisted task type/payload for playlist reconciliation
  is renamed; any `sync_playlist` task already persisted before this change
  deploys won't be recognized after it does; the daemon's existing
  no-handler-registered → retry → dead-letter handling absorbs this safely,
  but it's worth restarting with an otherwise-empty task queue if possible.

## Capabilities

### New Capabilities
- `playlist-reconciliation`: the recurring per-playlist job that, for a
  YouTube-linked playlist, diffs stored videos against YouTube's playlist
  contents, and, for every playlist, diffs the filesystem against recorded
  downloaded-video filenames and heals what it finds missing or orphaned.
  Supersedes `playlist-sync`.
- `custom-playlist-crud`: create a playlist with no YouTube playlist behind
  it, and add/remove individual videos on it by YouTube URL/ID, each
  verified and titled via the YouTube API before being accepted.

### Modified Capabilities
- `playlist-sync`: requirements retired and absorbed into
  `playlist-reconciliation` (recurring sync becomes recurring reconcile,
  scoped to YouTube-linked playlists for the membership-diff half).
- `playlist-crud`: playlists gain a `kind` distinguishing YouTube-linked from
  custom, changing what creation validates and what the response includes.
- `video-download`: a successful download additionally records the exact
  on-disk filename on the video, not just its quality.
- `video-cleanup`: file deletion is located by the video's recorded filename
  instead of a sanitized-title-plus-collision-suffix guess.

## Impact

- `domain/playlist`: `Playlist` gains a `kind` (`YoutubeLinked` vs. `Custom`);
  `PlaylistService::create_playlist` branches validation by kind (YouTube
  existence check vs. UUID format/uniqueness check) and gains video
  add/remove operations scoped to custom playlists.
- `domain/video`: `Video` gains a recorded filename, populated by
  `VideoService::download_video` on success and read by `delete_video_file`
  and the new reconciliation logic.
- `tasks/sync_playlist_task.rs`: renamed and reworked into a reconcile task
  covering both membership-diff and filesystem-diff.
- `infrastructure/repositories/filesystem_video_file_repository.rs`:
  matching simplifies from fuzzy title/suffix search to an exact filename
  lookup; gains directory-listing support for the reconciliation pass.
- New YouTube API repository for single-video lookup (existence + title),
  sibling to the existing playlist-exists and playlist-items repositories.
- `http/playlists`: new `custom_playlists` HTTP module, routes, and DTOs.
- SQLite schema: `playlists` gains a `kind` column; `videos` gains a
  `filename` column.
- OpenSpec: retires `playlist-sync`; adds `playlist-reconciliation` and
  `custom-playlist-crud`; modifies `playlist-crud`, `video-download`, and
  `video-cleanup`.
