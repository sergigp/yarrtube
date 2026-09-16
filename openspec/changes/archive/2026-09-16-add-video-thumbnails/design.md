## Context

See proposal.md - Why. Relevant existing shape:

- `ytdlp::download_video` (`src/infrastructure/shared/ytdlp.rs:52`) invokes
  `yt-dlp` once per video with an `-o "{base}.%(ext)s"` template and reads
  back the exact saved filename via `--print after_move:filename` (its last
  stdout line, in `--quiet` mode).
- `Video` (`src/domain/video/video.rs`) records `filename`/`quality` as
  verified facts, set by `mark_downloaded`, cleared by
  `reset_for_redownload`. `mark_downloaded` is called from exactly one
  production site: `VideoDownloader::download`
  (`src/domain/services/video_downloader.rs:63-68`).
- `VideoFileRepository` (`src/infrastructure/repositories/
  filesystem_video_file_repository.rs`) is already an injected port for
  listing/deleting files in an output directory, used by both reconcilers
  and by `VideoFileDeleter`.
- `video_reconciler.rs`/`channel_video_reconciler.rs` build a
  `downloaded_filenames` set from `Downloaded` videos' recorded `filename`
  and delete anything else in the output directory as an orphan
  (`video_reconciler.rs:236-239,293-300`) — README documents this as a hard
  "fully system-owned" invariant.
- `videos` table is `CREATE TABLE IF NOT EXISTS` with no migration
  framework (`sqlite_video_repository.rs:27-40`); the app's own DB file is
  ephemeral across restarts per the README (only the mounted video
  directory persists), so a new column only needs to be safe to add to a
  freshly-created table, not migrated in place.

## Goals / Non-Goals

**Goals:**
- Thumbnails survive filesystem reconciliation's orphan sweep and get
  cleaned up when their video is deleted, using the same recorded-fact
  pattern `filename` already uses.
- No new HTTP routes, no new external dependencies.

**Non-Goals:**
- Healing a thumbnail that goes missing on disk while its video stays
  healthy (see proposal.md and the prior conversation turn: left as-is,
  reconciliation only verifies the video file's health, not the
  thumbnail's).
- Backfilling thumbnails for videos downloaded before this change.
- Any SPA UI work.

## Decisions

**Derive the thumbnail's expected filename from the video's own filename,
verify it exists, then record it — don't parse a second yt-dlp print line.**
`yt-dlp` writes a converted thumbnail using the exact same output-template
base as the video (this is the mechanism the "same stem, `.jpg` extension"
convention relies on), so once `--convert-thumbnails jpg` is set, the
thumbnail's name is always `{video_filename_stem}.jpg` in the same output
directory. Reusing this instead of adding a second `--print` line avoids
depending on an unconfirmed/fragile yt-dlp print key for the thumbnail path,
and avoids widening `VideoDownloaderRepository::download`'s return type.
Concretely: `VideoDownloader` (already the only caller of
`mark_downloaded`) gains a `VideoFileRepository` dependency (already used
elsewhere for exactly this kind of check) and, after a successful download,
checks whether the derived thumbnail filename is present in `output_dir`
via `VideoFileRepository::list`/an existence check; if present, it's passed
to `mark_downloaded` as the recorded thumbnail filename, otherwise `None`.

**`thumbnail_filename` is a sibling field on `Video`, not derived on read.**
Chosen over re-deriving `<stem>.jpg` at every call site (reconciler, HTTP
DTO, deleter) because a verified, recorded fact can be `None` when the
thumbnail genuinely wasn't written (e.g. YouTube gave no thumbnail, or the
conversion step failed), whereas blind derivation can't distinguish "no
thumbnail" from "thumbnail missing" and would need a disk check duplicated
in three places anyway.

**Reconciliation's orphan-set includes `thumbnail_filename` but its
"healthy" check does not.** The existing "healthy" check
(`video_reconciler.rs:242-247`) only verifies the video file (present +
`.mp4`); it stays that way. Only the orphan-sweep skip-set grows to include
each `Downloaded` video's `thumbnail_filename`. This means a video whose
thumbnail alone disappears from disk is never detected or healed — see
Risks below, and the prior conversation turn where this trade-off was
raised and accepted explicitly rather than left implicit.

**`yt-dlp` flags added directly in `ytdlp.rs`, not made conditional.**
`--embed-thumbnail --write-thumbnail --convert-thumbnails jpg` are added
unconditionally to every download (alongside the existing
`--merge-output-format mp4`/`--remux-video mp4` flags), since `ffmpeg` is
already bundled in the runtime image and there's no per-playlist setting
that should disable thumbnails.

## Risks / Trade-offs

- **A lone missing thumbnail file is never healed** (see Non-Goals) →
  Accepted trade-off; the video itself stays correct and playable, only its
  poster art is stale/missing until the video is redownloaded for an
  unrelated reason.
- **Unconfirmed: whether a thumbnail-fetch failure (e.g. 404) can fail the
  whole `yt-dlp` process** → Needs verification during implementation
  against real `yt-dlp` behavior (spec's "Thumbnail unavailable for an
  otherwise successful download" scenario assumes it doesn't). If it turns
  out `yt-dlp` does hard-fail on a thumbnail error, `--no-abort-on-error` (or
  equivalent) may be needed so a flaky thumbnail never blocks the video
  itself from downloading.
- **Schema change with no migration framework** → New `thumbnail_filename`
  column added to the `CREATE TABLE IF NOT EXISTS` statement is safe for
  fresh DBs; since the DB is documented as ephemeral across restarts,
  there's no existing production row to migrate in place.
