## Context

See proposal.md - Why/What Changes for motivation and scope. Relevant current state:

- `VideoService::download_video` and `VideoService::delete_video_file` (`src/domain/video/service.rs`) both build `output_dir` as `Path::new(&self.videos_path).join(playlist.name.as_str())` — the playlist's display name doubles as its storage folder today.
- `PlaylistName` (`src/domain/playlist/playlist_name.rs`) rejects `/ \ : * ? " < > |` specifically because it's used as a single folder segment.
- `ytdlp::ensure_output_dir` already calls `std::fs::create_dir_all`, and `youtube_video_downloader_repository`/`filesystem_video_file_repository` already operate on whatever `Path` they're given — neither needs to change to support a multi-segment output directory. Confirmed: `Path::new("/videos").join("a/b/c")` produces `/videos/a/b/c` (std `Path::join` splits on `/`); `create_dir_all` creates every intermediate segment regardless of depth.
- `Quality` (`src/domain/playlist/quality.rs`) is currently owned by the playlist aggregate; `VideoService::download_video` already receives a `Quality` parameter (resolved once, at task-scheduling time, per the `add-playlist-download-quality` change) but never stores it.
- `playlists` and `videos` tables are created via `CREATE TABLE IF NOT EXISTS`, run unconditionally at startup, with no migration tooling — an established, accepted convention for this pre-1.0 project (see `add-playlist-download-quality`'s design.md Risks).

## Goals / Non-Goals

**Goals:**
- Decouple a playlist's storage location from its display name via a new, independently-validated `path`.
- Prevent path traversal: a `path` must never resolve outside the configured videos root.
- Record a video's actually-downloaded quality tier without adding any new HTTP surface (no video endpoints exist yet).
- Relocate `Quality` to `domain/shared/` cleanly, without changing its behavior (parsing, `as_str`, `Display`).

**Non-Goals:**
- No update-playlist endpoint, no drift detection between `playlist.quality` and `video.quality`, no redownload triggering — deferred to a later change per proposal.md.
- No migration tooling for existing SQLite files missing the new `path`/`quality` columns — same accepted risk as prior schema changes.
- No uniqueness enforcement across playlists' `path` values — `name` already permits collisions today (only `id` is unique), so this isn't a new class of risk, just an existing one now also possible via `path`.

## Decisions

**`PlaylistPath` is a new value object, not a reuse of `PlaylistName`'s validation.** `PlaylistName` forbids `/` outright (it's a single segment); `PlaylistPath` must *allow* `/` as a segment separator while rejecting:
- an empty path, or a path that trims to empty
- a leading `/` (absolute path — would replace the videos root entirely under `Path::join`, escaping it completely)
- any `..` segment (parent traversal)
- any empty segment, i.e. `//` anywhere, or a leading/trailing `/` on an otherwise valid path (`a//b`, `/a/b`, `a/b/`)
- the same filesystem-unsafe characters `PlaylistName` rejects (`\ : * ? " < > |`), checked per segment, since each segment is still a folder name

Alternative considered: reuse `PlaylistName`'s validator per `/`-split segment instead of building a new type. Rejected: `PlaylistPath` needs `/` structurally significant (a separator, not a rejected character), while `PlaylistName` treats it as simply forbidden — different validation shapes, not a parametrization of the same one. Keeping them separate types also matches this codebase's existing pattern of one value object per validated concept.

**`path` is required on create, with no default derived from `name`.** Confirmed with the user: decoupling should be explicit, not implicit — a caller always states where a playlist lives on disk, same as it always states `name` and `quality`. This is a breaking change to `CreatePlaylistRequest`, accepted the same way the original `quality` addition was (pre-1.0, no deployments, no back-compat shim).

**Duplicate-ID creation ignores a re-supplied `path`, mirroring `quality`'s existing idempotency behavior.** Both `name`, `quality`, and now `path` are "set once at creation" fields; an idempotent create keyed on YouTube playlist ID already ignores a differing `quality`, so `path` follows the same rule for consistency rather than introducing a different reconciliation policy for one field.

**`Quality` moves to `domain/shared/quality.rs`, kept as the same enum/API.** `Video` needs it and shouldn't depend on `domain/playlist/`'s internals (aggregates own their own state; a shared value object used by two aggregates belongs in `domain/shared/`, matching where `PlaylistId`/`VideoId` already live). This is a pure move — `Quality::new`, `as_str`, `Display`, and all existing tests carry over unchanged; every `use crate::domain::playlist::Quality` import across the codebase (service.rs, task.rs, ytdlp.rs, youtube_video_downloader_repository.rs, HTTP DTOs, and their tests) updates to `use crate::domain::shared::Quality`.

**`Video::mark_downloaded` takes the resolved `Quality` as a parameter**, set by `VideoService::download_video` from the same `quality: Quality` it already receives (originally resolved by `DownloadVideoOnVideoAdded` at task-scheduling time — unchanged). No new lookups needed; this just captures a value the method already has.

**`videos.quality` is a nullable column**, since a video only has a recorded quality once it's been successfully downloaded at least once; pending/in-progress/errored videos have none.

## Risks / Trade-offs

- [`CreatePlaylistRequest.path` becomes required, breaking any existing caller] → Acceptable: pre-1.0, no real deployments, matches how `quality` was introduced as a breaking addition previously.
- [No uniqueness check on `path` — two playlists could resolve to the same directory and their downloads collide] → Not a new risk: `name` already permits duplicates today with the same effect. Left as a pre-existing, unaddressed risk rather than scope creep for this change.
- [`path` validation must correctly block absolute paths and `..` given `std::path::Path::join`'s replace-on-absolute behavior] → Mitigated by validating in the value object constructor (`PlaylistPath::new`) before any path ever reaches `Path::join`, not by relying on join-time behavior.
- [Relocating `Quality` touches many files (any current importer of `domain::playlist::Quality`)] → Mechanical, low-risk: no behavior change, just import-path updates; covered by the existing test suite (all `Quality` tests move with the type and continue passing unchanged).
