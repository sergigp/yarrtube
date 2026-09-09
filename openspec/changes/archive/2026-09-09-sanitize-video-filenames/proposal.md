## Why

Downloaded video filenames are unusable: they're `yt-dlp`'s raw, unedited YouTube title plus a `[VIDEO_ID]` suffix, so they carry extra/inconsistent whitespace, filesystem-unsafe characters that `yt-dlp` half-heartedly swaps for lookalike Unicode punctuation, decorative symbols/emoji, and unbounded length. Now is the right time to fix this — the project is still in alpha, so there's no backlog of files that need migrating, and naming behavior is only going to grow more elaborate (e.g. future Plex-style `S01E01` episode numbering), so the sooner this has a proper seam the less rework later.

## What Changes

- Introduce a new `video-naming` capability that derives a sanitized, safe output filename from a video's title, used by every download path instead of `yt-dlp`'s default title-based naming.
- Sanitization is objective only: collapse/trim whitespace, replace filesystem-unsafe characters, strip decorative/emoji symbols (while preserving accented letters), and truncate to a filesystem-safe length. No casing normalization or clickbait-phrase stripping — those are subjective and risk mangling meaningful content (e.g. real acronyms like `AI`, `RAG`, `RLHF`).
- Drop the `[VIDEO_ID]` suffix from the filename by default. It is appended back only as a fallback, when the sanitized name collides with a file already present in the same playlist output directory.
- Both existing download paths — the `playlist-download` CLI command and the `video-download` daemon path — now pass this computed filename explicitly to `yt-dlp` (via its output template) instead of relying on `yt-dlp`'s own default naming.
- No migration of already-downloaded files; this only affects videos downloaded from this change forward.
- Not implemented in this change, but designed for: additional naming strategies (e.g. per-playlist episode numbering for Plex) plugging into the same seam without reworking the download plumbing.

## Capabilities

### New Capabilities
- `video-naming`: derives a sanitized, collision-safe output filename (no extension) from a video's title, for use by any download path.

### Modified Capabilities
- `playlist-download`: the "Sequential Video Download" requirement currently states the system relies entirely on `yt-dlp`'s default behavior with no flags. That guarantee is narrowed to format/quality selection only — output filenames now follow the `video-naming` capability instead of `yt-dlp`'s default title-based naming.

## Impact

- `src/domain/video/`: new value object that sanitizes a raw title into a safe base filename (pure function, no I/O), mirroring the existing `PlaylistName` pattern.
- `src/infrastructure/shared/ytdlp.rs`: `download_video` gains an explicit desired filename, resolves collisions against the output directory's existing files, and passes the final name to `yt-dlp` via `-o`.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs` and `src/infrastructure/client/youtube_downloader_client.rs`: call sites thread the computed filename through.
- `src/domain/video/service.rs`: `VideoService::download_video` computes the sanitized filename from `Video.title` before invoking the downloader port.
- No database schema changes — `Video` does not store a file path today and still won't.
- Playlist/directory naming (`PlaylistName`) is unaffected — this change is scoped to video filenames only.
