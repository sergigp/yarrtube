## Context

A video's folder name is decided lazily, at real-download time, by
`resolve_folder_collision` in `ytdlp.rs` (checks what already exists on
disk). `Video.filename`/`Video.thumbnail_filename` are both set together, in
one place, by `VideoDownloader::download` on success. Three call sites
create a `Video` row today, all synchronously: `VideoReconciler::
sync_playlist_membership`, `ChannelVideoReconciler::sync_channel_membership`,
`CustomPlaylistVideoAdder::add`. See proposal.md for why an early,
independent thumbnail fetch is needed.

## Goals / Non-Goals

**Goals:**
- Fetch a thumbnail at video-creation time, best-effort, with zero new
  `Task`/`DomainEvent` variants and zero schema change.
- Guarantee the real download reuses the exact folder the early fetch
  created, never a second, differently-suffixed one.

**Non-Goals:**
- Redownloading a fresh thumbnail when reconcile resets a video for
  redownload (missing file, permanently errored) — that video keeps
  whatever thumbnail it already has; only a video with *no* thumbnail at
  all gets the new recovery pass.
- Choosing between `yt-dlp` and a direct CDN fetch — already decided
  (`yt-dlp --skip-download`).

## Decisions

**No new `Video` column for the folder.** The per-video folder name is
always the first path segment of `thumbnail_filename` (e.g.
`"My Video/My Video.jpg"` → `"My Video"`) — exactly what `top_level_entry`
(added by `folders-per-video-storage`) already extracts. The real download
recovers it from there; when `thumbnail_filename` is `None` (fetch never
ran or failed), it falls back to today's collision-resolving behavior
unchanged.

**Thumbnail fetch is a new pure `ytdlp.rs` function**, sibling to
`download_video`, reusing `resolve_folder_collision`,
`ensure_output_dir`, `output_retrying_busy`, and
`remove_video_dir_best_effort`. Unlike `download_video`, a successful exit
with no printed filename means "no thumbnail for this video" (`Ok(None)`),
not a systemic error — a thumbnail is optional even on a clean `yt-dlp`
run.

**One domain service, `ThumbnailFetcher`, called from all 3 creation
sites plus both reconcilers' recovery pass.** It never returns `Err` to
its caller — every failure is logged and swallowed internally, so callers
never need their own try/catch-and-ignore boilerplate.

## Files

- `src/infrastructure/shared/ytdlp.rs` — new `fetch_thumbnail` fn + `FetchedThumbnail` struct; `download_video` gains an `existing_folder: Option<&str>` param.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs` — trait gains `fetch_thumbnail`; `download` gains `existing_folder`; both impls (real + fake) updated.
- `src/domain/video/video.rs` — new `with_thumbnail` transition.
- `src/domain/services/thumbnail_fetcher.rs` — new `ThumbnailFetcher` service.
- `src/domain/services/video_downloader.rs` — resolves `existing_folder` from `video.thumbnail_filename` via `top_level_entry` before calling `download`.
- `src/domain/services/video_reconciler.rs` — calls `ThumbnailFetcher` on new-video persistence and as a missing-thumbnail recovery pass; widens `protected_top_level`.
- `src/domain/services/channel_video_reconciler.rs` — same three changes, channel-side.
- `src/domain/services/custom_playlist_video_adder.rs` — calls `ThumbnailFetcher` on add; gains a `videos_path` field.
- `src/serve.rs` (application wiring) — constructs `ThumbnailFetcher` and threads it into the three services above.

## Types & Signatures

```rust
// ytdlp.rs
pub struct FetchedThumbnail {
    pub folder: String,
    pub filename: String,
}

pub fn fetch_thumbnail(
    ytdlp_path: &Path,
    video_url: &str,
    desired_filename: &str,
    video_id: &str,
    output_path: &Path,
) -> Result<Option<FetchedThumbnail>>;

pub fn download_video(
    ytdlp_path: &Path,
    video_url: &str,
    desired_filename: &str,
    video_id: &str,
    quality: Quality,
    output_path: &Path,
    existing_folder: Option<&str>,
) -> Result<Option<DownloadedVideo>>;
```

```rust
// youtube_video_downloader_repository.rs
pub trait VideoDownloaderRepository: Send + Sync {
    fn download(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        quality: Quality,
        output_dir: &Path,
        existing_folder: Option<&str>,
    ) -> anyhow::Result<Option<DownloadedVideo>>;

    fn fetch_thumbnail(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        output_dir: &Path,
    ) -> anyhow::Result<Option<FetchedThumbnail>>;
}
```

```rust
// domain/video/video.rs
impl Video {
    pub fn with_thumbnail(self, thumbnail_filename: impl Into<String>, now: DateTime<Utc>) -> Self;
}
```

```rust
// domain/services/thumbnail_fetcher.rs
#[derive(Clone)]
pub struct ThumbnailFetcher {
    video_repository: Arc<dyn VideoRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    clock: Arc<dyn Clock>,
}

impl ThumbnailFetcher {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self;

    /// Best-effort: fetches `video`'s thumbnail into `output_dir` and
    /// persists it via `with_thumbnail`. Logs and returns on any failure,
    /// no-ops if `video.thumbnail_filename` is already set.
    pub fn fetch(&self, video: &Video, output_dir: &Path);
}
```

## Call Stack

**Video creation (3 sites, same shape):**
```
sync_playlist_membership(playlist)                 [VideoReconciler]
  Video::create(youtube_id, title, now) -> video
  video_repository.save(&video)
  thumbnail_fetcher.fetch(&video, &output_dir)      output_dir = videos_path/playlist.path
  event_publisher.publish(VideoAddedToPlaylist)

sync_channel_membership(channel)                    [ChannelVideoReconciler]
  Video::create(...) -> video
  video_repository.save(&video)
  thumbnail_fetcher.fetch(&video, &output_dir)      output_dir = videos_path/channel.path
  event_publisher.publish(VideoAddedToChannel)

CustomPlaylistVideoAdder::add(playlist_id, video_id)
  youtube_video_repository.find(video_id) -> title
  Video::create(...) -> video
  video_repository.save(&video)
  thumbnail_fetcher.fetch(&video, &output_dir)      output_dir = videos_path/playlist.path
  event_publisher.publish(VideoAdded)
```

**Inside `ThumbnailFetcher::fetch`:**
```
fetch(video, output_dir)
  if video.thumbnail_filename.is_some() -> return   (already have one; used by recovery pass too)
  filename = VideoFilename::from_title(&video.title)
  video_downloader_repository.fetch_thumbnail(
      video.youtube_id.to_url(), filename.as_str(), video.youtube_id.as_str(), output_dir)
  Ok(Some(fetched)) ->
      thumbnail_filename = format!("{}/{}", fetched.folder, fetched.filename)
      video_repository.update(&video.clone().with_thumbnail(thumbnail_filename, clock.now()))
  Ok(None) | Err(_) -> warn!(...), leave video untouched
```

**Real download, reusing the folder:**
```
VideoDownloader::download(video_id, quality, output_dir, is_last_attempt)
  video = video_repository.find(video_id)
  existing_folder = video.thumbnail_filename.as_deref().map(top_level_entry)
  video_downloader_repository.download(
      url, filename.as_str(), youtube_id, quality, output_dir, existing_folder)
  ... unchanged from here (mark_downloaded, generate_metadata, etc.)
```

**Missing-thumbnail recovery (new step in each reconcile pass):**
```
run_reconcile_pass(playlist) / (channel)
  ... existing membership sync + filesystem reconciliation ...
  for video in stored_videos where video.thumbnail_filename.is_none():
      thumbnail_fetcher.fetch(&video, &output_dir)
```

## Risks / Trade-offs

- **Reconcile passes take longer on a large first sync.** Each newly
  discovered video now pays one extra `yt-dlp` spawn (~1-2s) inline,
  sequentially, inside `sync_playlist_membership`/`sync_channel_membership`.
  Accepted — see proposal.md.
- **`existing_folder` bypasses the collision check.** If a caller ever
  passed a folder name that collides with an unrelated entry, `download_video`
  would silently write into it. Mitigated: `existing_folder` only ever comes
  from `top_level_entry(video.thumbnail_filename)`, i.e. a folder *this same
  video* created moments earlier — never a guess.

## Migration Plan

No data migration. A video already `Downloaded` or mid-flight before this
ships is unaffected (its `thumbnail_filename`, if any, already points at its
real folder, so `existing_folder` resolution is a no-op for it). Rollback is
a plain code revert; no on-disk state written by the new code needs undoing
— a pre-fetched thumbnail folder for a still-`Pending` video is just a
normal per-video folder indistinguishable from one the old code would have
created at download time.
