## Context

See `proposal.md` - Why for motivation. Relevant current state:

- Both download paths — `playlist-download` (CLI, `YtDlpDownloaderClient::download_all`) and `video-download` (daemon, `VideoService::download_video`) — funnel into the single low-level `infrastructure/shared/ytdlp.rs::download_video(video_url, output_path)`, which runs `yt-dlp <url>` with `output_path` as its working directory and no `-o` template, so `yt-dlp` falls back to its own default `%(title)s [%(id)s].%(ext)s` naming.
- The `Video` domain entity already holds `title`; it stores no file path today and won't need to.
- `PlaylistName` (`domain/playlist/`) already defines the filesystem-unsafe character set it rejects. This change reuses that same character list, but replaces instead of rejects, since a video's title is external metadata yarrtube doesn't control and can't refuse.

## Goals / Non-Goals

**Goals:**
- One deterministic, pure function that turns a raw video title into a safe base filename.
- Both existing download call sites use it without `ytdlp.rs` needing to know anything about titles.
- Leave a seam so a future naming strategy (e.g. per-playlist Plex episode numbering) can replace this function later without reworking the download plumbing.

**Non-Goals:**
- Clickbait detection, casing normalization, or any other subjective title rewriting (see proposal).
- Renaming files already on disk.
- A configurable/pluggable naming-strategy interface. Only one strategy exists after this change — the seam is structural (one call site, one function), not a runtime-switchable trait, until a second strategy actually needs one.

## Decisions

**1. Sanitization is a plain function in `domain/video/`, not a trait/strategy interface.**
Only one implementation exists. A `VideoNamingStrategy`-style trait today would be guessing at a shape before a second implementation (e.g. episode numbering, which needs playlist-order context this function doesn't have) proves it. Extracting a trait later, once that second strategy actually lands, is a small mechanical refactor because every caller already goes through one function. Keeps this change simple without making the next one harder.

**2. The filename is computed once, at each call site that already holds the title (`VideoService::download_video` and `YtDlpDownloaderClient::download_all`), and passed into `ytdlp::download_video` as an explicit parameter.**
Both call sites already have the title in hand; no new data needs to flow through the system. This keeps `ytdlp.rs` a pure "run the process" adapter that's told what name to use, rather than an adapter that reaches back into domain concerns to derive one. Matches this codebase's layering: domain decides, infrastructure executes.

**3. Collision detection and the final `-o` template construction happen in `ytdlp.rs::download_video`, immediately before invoking the process.**
Detecting a collision means listing the output directory's existing files — I/O, which belongs in infrastructure. It's also the only place that knows the final extension isn't chosen until `yt-dlp` picks a format, so the check is "does any file whose stem equals the desired base name already exist," not an exact path match. On collision, the video ID is appended and that becomes the final base passed to `-o "<base>.%(ext)s"`.

**4. Truncate the sanitized base to 150 bytes (not characters), on a UTF-8 boundary, before any collision suffix is appended.**
Common filesystems cap filenames at 255 bytes. 150 bytes leaves comfortable room for the longest realistic extension plus a possible ` [VIDEO_ID]` collision suffix, even with multi-byte UTF-8 titles. Truncating by bytes (not `chars().take(n)`) avoids letting a too-long multi-byte title through while still bounding total byte length, and slicing must land on a char boundary to avoid producing invalid UTF-8.

**5. Decorative symbol stripping is Unicode-category-based (drop `Symbol` categories `So`/`Sk` and common emoji blocks; keep `Letter`, `Number`, `Mark`, and standard ASCII punctuation/space), not an ASCII-only allowlist.**
Real titles mix legitimate non-ASCII letters (accented Spanish words) with decorative junk (emoji, arrows, the fullwidth punctuation `yt-dlp` itself substitutes). An ASCII-only filter would mangle real words; category-based filtering separates "letter/number/normal punctuation" from "decorative" without a hand-maintained character list.

## Risks / Trade-offs

- **Collision check adds a directory listing per download** → negligible: playlist directories hold at most a few hundred files, and this happens once per video download, which itself takes seconds to minutes.
- **Category-based symbol stripping is a judgment call and could occasionally drop a character someone wanted kept** (e.g. `&`, a math symbol in a title) → mitigated by keeping the rule conservative (only `So`/`Sk` + emoji blocks, not all non-ASCII); still a clear improvement over today's unfiltered defaults.
- **Dropping the video ID by default means a collision is only checked against files present in the directory at invocation time** → matches `yt-dlp`'s own existing assumption (it also treats the directory listing at invocation time as authoritative) and is no worse than the status quo.

## Migration Plan

Not applicable — per the proposal's scope decision, no existing files are touched. The new naming behavior takes effect the next time each download path runs.
