## Files

- `src/infrastructure/shared/ytdlp.rs` — `probe_captions` collects stderr warnings; filtering yt-dlp-specific wording belongs in the yt-dlp adapter, so the domain keeps treating `CaptionProbe.warnings` as "caption-relevant warnings".
- `src/domain/services/subtitle_fetcher.rs` — doc comment on `log_inconclusive` only; logic unchanged.

## Types & Signatures

```rust
pub struct CaptionProbe {
    // ...
    /// yt-dlp's stderr `WARNING:` lines that can affect caption tracks
    /// (cache write failures and skipped-formats notices dropped).
    pub warnings: Vec<String>,
}

/// `WARNING:` lines of a caption probe's stderr, minus harmless ones.
fn caption_warning_lines(stderr: &str) -> Vec<String>;   // replaces warning_lines

/// Cache write failure (`Writing cache to '...' failed`) or a
/// `formats have been skipped` notice (SABR): never about captions.
fn is_harmless_probe_warning(line: &str) -> bool;
```

Deny-list, not allow-list: an unknown warning keeps the probe inconclusive, so a new yt-dlp caption warning can never wrongly mark a video Unavailable.

## Call Stack

```
FetchSubtitlesTask::run
  -> SubtitleFetcher::fetch_and_place(video, output_dir)
    -> VideoDownloaderRepository::probe_captions(url, staging)
      -> ytdlp::probe_captions(ytdlp_path, url, staging)
        -> caption_warning_lines(stderr)
          -> is_harmless_probe_warning(line)   // per WARNING: line
    -> probe.tracks.original() == None
       -> warnings empty   => mark_unavailable_if_long_missing
       -> warnings present => log_inconclusive
```

## Test Plan

1. Behaviour tests: none new. Application behaviour is unchanged for a given `CaptionProbe`; existing `it_should_keep_subtitles_missing_when_the_downloader_warns` and the no-track/unavailable tests in `fetch_subtitles_task.rs` cover both branches.
2. Infrastructure — `ytdlp::probe_captions` (fake yt-dlp script):
   - `it_should_drop_a_cache_write_warning_from_a_probe` — stderr with `WARNING: Writing cache to '/home/x/.cache/yt-dlp/youtube-sigfuncs/a.json' failed: Traceback ...` → `warnings == vec![]`.
   - `it_should_drop_a_formats_skipped_warning_from_a_probe` — stderr with the `web_embedded client https formats have been skipped ... SABR` warning → `warnings == vec![]`.
   - `it_should_keep_a_caption_warning_next_to_harmless_ones_in_a_probe` — stderr with cache, formats-skipped and `Some subtitles may be missing` → `warnings == vec!["WARNING: [youtube] yt1: Some subtitles may be missing"]`.
