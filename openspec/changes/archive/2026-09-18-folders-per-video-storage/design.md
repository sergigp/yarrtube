## Context

Today a container's (playlist's or channel's) output directory is flat: `{videos_root}/{container.path}/{sanitized_title}.mp4` plus a `{sanitized_title}.jpg` sibling. Collision between two videos whose titles sanitize identically is resolved by `resolve_collision` in `src/infrastructure/shared/ytdlp.rs`, which scans the directory for an existing file with the same stem and appends ` [{video_id}]` when found. `Video.filename`/`Video.thumbnail_filename` (`src/domain/video/video.rs`) are plain `Option<String>` bare filenames, and every consumer (deletion, reconciliation, HTTP media serving) resolves the full path as `{videos_root}/{container.path}/{video.filename}`. See proposal.md for why this needs to change.

## Goals / Non-Goals

**Goals:**
- Every downloaded video gets its own folder (video file + thumbnail + empty `meta.nfo`), named via the existing sanitized-title logic.
- Zero database schema change, zero HTTP API/DTO shape change, zero SPA-visible behavior change.
- Videos already downloaded under the old flat layout keep working exactly as before, indefinitely — no migration step, no forced redownload.

**Non-Goals:**
- Migrating existing flat-layout files into folders.
- Populating `meta.nfo`'s content (stays empty; a future change's scope).
- Changing how a video's *filename itself* (the sanitized title) is derived — `VideoFilename::from_title` is unchanged.

## Decisions

### Store the folder as part of `filename`/`thumbnail_filename`, not as a new column
`Video.filename`/`thumbnail_filename` become "a path relative to the container's output directory that may contain one `/`" (e.g. `"My Video/My Video.mp4"`) instead of always being a bare filename. Every existing consumer that does `output_dir.join(video.filename)` (HTTP media serving via `ServeDir`, `VideoFileRepository::delete`, reconciliation) already resolves an arbitrary relative path correctly, multi-segment or not — `ServeDir` nested-path support is already covered by an existing test (`it_should_serve_a_channel_owned_videos_file_under_its_nested_storage_path` in `src/serve.rs`). This avoids a migration, a schema change, a DTO change, and any SPA change beyond fixing URL-encoding of the (now sometimes multi-segment) filename.

Alternative considered: add a `video_folder` column and a full-path value object. Rejected — much larger blast radius (schema migration, DTOs, every read site) for no behavioral benefit, given the relative-path-string approach already works end to end.

### Folder-name collision resolution mirrors today's file-collision resolution, unchanged in spirit
`resolve_collision`'s check moves from "does a file with this stem exist in the container dir" to "does an entry with this exact name exist in the container dir" (`output_path.join(desired_filename).exists()`), and on collision appends ` [{video_id}]` to the *folder* name instead of the *file* name — same fallback shape, same determinism. Because the video's own file lives alone in its own folder, the file itself no longer needs a per-file collision suffix; the folder name is reused as the file's base name too (`{folder}.%(ext)s`).

Retries are safe by construction: the appended suffix is always this video's own YouTube ID, so a retry of the same video (which may find its own from a previous partial attempt still on disk) deterministically resolves to the same folder name every time — this already held for the old file-stem check and continues to hold here.

### `top_level_entry()` helper unifies old and new layouts for deletion/reconciliation
A single pure function, `top_level_entry(relative_path: &str) -> &str`, returns the first path component of a stored `filename`/`thumbnail_filename`. For a new-style `"folder/file.ext"` path that's the folder; for a legacy flat `"file.ext"` path (no `/`) it's the filename itself, unchanged. Both `VideoFileDeleter::delete_video_file` and the two reconcilers' orphan-detection use this to compute "the thing that owns this stored path, at the container-directory level" without needing to branch on which layout generation a given video belongs to.

### `VideoFileRepository::delete` becomes directory-aware instead of adding a new method
`delete(output_dir, name)` currently assumes `name` is a file (`std::fs::remove_file`). It's extended to `symlink_metadata` first and dispatch to `remove_dir_all` for a directory, `remove_file` otherwise — same signature, same "missing is not an error" contract. Every existing caller that deletes "the top-level entry for this video" (via `top_level_entry()`) then correctly removes a whole per-video folder without any call-site change. A new infallible `file_exists(output_dir, filename) -> bool` method is added alongside it, used by the reconcilers' per-video health check, since that check needs to test a *specific* (possibly nested) recorded path directly rather than intersect against a top-level directory listing (which, post-change, lists folder names, not nested files).

### Reconciliation health check and orphan sweep, updated for nesting
- Health check: `files.iter().any(|f| f == filename)` (broken for a nested `"folder/file.mp4"` path, since `files` is a top-level listing) becomes `video_file_repository.file_exists(&output_dir, filename)`. The existing `.mp4` extension check is untouched — `Path::extension()` already only looks at the last path component.
- Orphan sweep: `protected_filenames` (today a flat set of raw `filename`/`thumbnail_filename` strings, compared directly against top-level entries) becomes `protected_top_level`, built by mapping every recorded `filename`/`thumbnail_filename` through `top_level_entry()` first. The sweep loop itself (`for file in &files { if !protected... { delete(...) } }`) is otherwise unchanged — its `delete()` call is now directory-aware, so a genuinely orphaned video folder is removed whole rather than erroring on `remove_file` against a directory.

## Risks / Trade-offs

- **Mixed layouts coexist indefinitely.** A container directory can have some video folders and some flat legacy files side by side forever, since there's no migration. Mitigation: every consumer (`top_level_entry`, `file_exists`, directory-aware `delete`) is written to handle both shapes uniformly, and this is exercised by dedicated tests for the legacy-flat case in each changed unit, not just the new-style case.
- **`ServeDir` `%2F` handling.** The SPA fix relies on percent-encoding the filename segment-by-segment (`filename.split('/').map(encodeURIComponent).join('/')`) rather than encoding the whole string as one blob, so the request path naturally contains a literal `/` per segment (mirroring how `playlistPath` is already encoded) instead of a `%2F` that `ServeDir` would need to decode specially. This is verified against the existing nested-media-serving test rather than assumed.
- **Retry/partial-download folder reuse.** If a download attempt fails after creating the video's folder and writing `meta.nfo` but before finishing the video file, a retry's folder-collision check will see that folder as "already existing" and append the video's own ID to the (now once-more-checked) name — deterministic and self-consistent (see Decisions above), but means a retried video's final folder name can end up suffixed with its ID even though no *other* video ever collided with it. This already matches the pre-existing behavior class for file-level retries, so it's not a regression, just carried over.

## Migration Plan

No data migration. This is a forward-only change to how *new* downloads are laid out on disk; deploying it requires no backfill, no downtime step, and no coordination with existing on-disk state. Rollback is a plain code revert — no on-disk state written under the new layout needs to be undone for old code to keep working, since old code never reads a `filename` containing `/` in the first place (it wasn't possible before this change), and any video downloaded under the new layout before a rollback would simply be treated by the reverted code as unhealthy on the next reconcile pass and redownloaded under the old flat layout.
