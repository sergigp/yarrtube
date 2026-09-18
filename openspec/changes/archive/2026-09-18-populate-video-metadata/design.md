## Context

See `proposal.md` for motivation. Relevant current state:

- `ytdlp::download_video` (`src/infrastructure/shared/ytdlp.rs`) creates a
  video's output folder and writes an empty `meta.nfo` into it before
  invoking `yt-dlp`. It is a pure process-invocation module with no HTTP
  client and no YouTube API knowledge, tested via a fake `yt-dlp` shell
  script.
- `VideoDownloader::download` (`src/domain/services/video_downloader.rs`)
  loads the `Video`, calls `VideoDownloaderRepository::download`, and on
  success looks up the thumbnail filename already written to disk before
  marking the video `Downloaded`.
- `YoutubeVideoRepository` (`src/infrastructure/repositories/
  youtube_video_repository.rs`) already calls `videos.list?part=snippet`
  and returns only `title`, used solely by `CustomPlaylistVideoAdder`. It
  is left untouched by this change.
- `Video` (`src/domain/video/video.rs`) has no description, tags,
  channel, publish date, or category fields, and this change does not add
  any — that data lives only in the new `VideoMetadata` entity.
- `PlaylistVideo.position: Option<i64>` reflects a YouTube-linked
  playlist's actual order and only changes when the playlist itself is
  reordered. `ChannelVideo.position: i64` is a recency rank recomputed
  from scratch on every reconcile pass — every existing video's position
  shifts whenever the channel publishes something new.
- `PlaylistVideoRepository`/`ChannelVideoRepository` already expose
  `find_by_video(&VideoRecordId)`, so the container a video belongs to,
  and its position, can be looked up at download or reconcile time without
  threading anything through `Task::DownloadVideo`'s payload.
- No XML-writing code or crate exists in the repo.

## Goals / Non-Goals

**Goals:**
- Generate a real `movie.nfo` per video from YouTube's own data, at
  download time, with no schema growth on `Video` itself.
- Make a failed or skipped generation self-healing via reconcile, without
  needing to redownload the video.
- Keep `ytdlp.rs` a dumb process-invocation layer; keep the domain layer
  ignorant of *where* metadata is persisted.

**Non-Goals:**
- Storing YouTube metadata for reuse elsewhere (e.g. a future web UI
  detail page) is not a design target here, though the chosen storage
  shape doesn't preclude it later.
- Triggering a Plex library scan.
- Backfilling or migrating videos downloaded under the previous
  blank-`meta.nfo` layout.

## Decisions

### A new `YoutubeMetadataRepository` port, separate from `YoutubeVideoRepository`
`YoutubeVideoRepository::find` exists for a different purpose
(confirming a video's existence/title when adding it to a custom
playlist) with a different caller. Rather than widening its return type
and coupling that caller to fields it doesn't need, add a new port:

```rust
pub trait YoutubeMetadataRepository: Send + Sync {
    fn find(&self, id: &VideoId) -> anyhow::Result<Option<YoutubeMetadata>>;
}
pub struct YoutubeMetadata {
    pub title: String,
    pub description: String,
    pub channel_title: String,
    pub published_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub category_id: Option<String>,
}
```
Its adapter (`YoutubeApiMetadataRepository`) calls the same
`videos.list?part=snippet` endpoint as `YoutubeVideoRepository`, reusing
the already-configured `YOUTUBE_API_KEY`. Both repositories are
constructed independently in `serve.rs`.

**Alternative considered**: extend `YoutubeVideo`/`YoutubeVideoRepository`
in place. Rejected — it would make an unrelated caller's return type grow
for a concern it doesn't have, for no real code-sharing benefit beyond the
one HTTP call shape.

### `VideoMetadataRepository`: one domain-facing port, one composed implementation
Domain only knows it is "saving metadata" — where and how many places
that lands is entirely an infrastructure concern:

```rust
pub trait VideoMetadataRepository: Send + Sync {
    fn save(&self, metadata: &VideoMetadata, video_dir: &Path) -> anyhow::Result<()>;
    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoMetadata>>;
}

pub struct SqliteVideoMetadataRepository { connection: Arc<Mutex<Connection>> }

impl VideoMetadataRepository for SqliteVideoMetadataRepository {
    fn save(&self, metadata: &VideoMetadata, video_dir: &Path) -> anyhow::Result<()> {
        self.write_movie_nfo(metadata, video_dir)?;  // file write, first
        self.save_row(metadata)                       // db write, only once the file succeeded
    }
    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoMetadata>> {
        self.find_row(video_id)                        // db-only, never touches the file
    }
}
```

The database row is the single source of truth for "has this video's
metadata been generated" — `find` never inspects `movie.nfo`. This means
completeness detection never needs to parse XML back into an entity, and
naturally self-heals a process crash between the file write and the DB
write (no row recorded → treated as not populated → regenerated next
reconcile pass, which simply overwrites the file again; harmless).

Method names (`save`/`find`) match the existing convention used across
this codebase's other repositories (e.g. `VideoRepository`,
`ChannelVideoRepository`, both of which already use `save` as an
insert-or-replace).

**Alternative considered**: two separate ports (a file-writer and a
DB-metadata-repository) explicitly sequenced by the domain caller.
Rejected per direct feedback — the ordering invariant is an
implementation detail of "how metadata gets persisted," not a domain
decision, so it belongs inside the one composed adapter.

### Failure handling: skip the save entirely, never fail the download
If `YoutubeMetadataRepository::find` fails or returns `None` after a
successful `yt-dlp` download, the orchesting step (in
`VideoDownloader::download`, after the existing thumbnail lookup) simply
does not call `VideoMetadataRepository::save` at all — no `movie.nfo` is
written (not even a blank one) and no DB row is created. The video is
still marked `Downloaded`, exactly as it is today. The reconcile repair
pass (see below) picks it up automatically on the next pass.

**Alternative considered**: write a blank `movie.nfo` as a marker (mirrors
today's placeholder behavior). Rejected once the DB row became the
completeness signal — an empty file would be redundant, and skipping the
write avoids ever having a partially-populated or blank file sitting in a
video's folder mid-repair-cycle.

### Reconcile-time repair loop
Both `VideoReconciler::reconcile_filesystem` and
`ChannelVideoReconciler`'s equivalent already loop over `Downloaded`
videos checking file health. A parallel loop is added, doing the same
generation steps as the download-time path (fetch `YoutubeMetadata`,
resolve `sorttitle`, build `VideoMetadata`, `save`) for any `Downloaded`
video for which `VideoMetadataRepository::find` returns `None` — no
status change, no re-download, no `reset_for_redownload`. Both
`VideoReconciler` and `ChannelVideoReconciler` gain the two new
repository dependencies as constructor arguments, following the existing
`Arc<dyn Trait>` injection pattern (both already use
`#[allow(clippy::too_many_arguments)]`).

There is no bounded-retry/backoff for this repair — it simply runs again
every reconcile interval until it succeeds. Acceptable for a self-hosted,
single-user tool; flagged as a known property, not a gap.

### Sorttitle source: playlist position when stable, publish date otherwise
`PlaylistVideo.position` is used directly (zero-padded) when present.
`ChannelVideo.position` is deliberately **not** used for `sorttitle` — it
is a recency rank recomputed on every reconcile pass, so a value baked
into a file written once at download (or repair) time would drift stale
as soon as the channel published something new. Channel-tracked videos,
and custom-playlist videos (which have no recorded position at all), use
a zero-padded prefix derived from the video's own immutable
`publishedAt` date instead — a value that, once written, is stable
forever, by construction.

### Genre mapping: hardcoded table
A `match category_id { ... }` table of the handful of YouTube category
IDs actually encountered, rather than caching `videoCategories.list`.
There is no existing caching/scheduled-refresh mechanism anywhere in this
codebase, and the category list is small and effectively static; adding
one for this alone isn't justified. An unmapped ID simply omits `<genre>`.

### XML serialization
Add a `quick-xml` dependency (or equivalent) rather than hand-building
XML strings, so escaping is guaranteed correct rather than manually
tracked per field.

## Risks / Trade-offs

- [Reconcile repair adds two new dependencies to two already-large
  reconciler constructors] → Mitigated by following the exact pattern
  already used for every other dependency on those types; no new
  wiring style introduced.
- [Unbounded repair retries on a permanently-broken video (e.g. deleted
  from YouTube after download)] → Acceptable: it costs one API call per
  reconcile interval per such video, and the video still has a working
  `.mp4`/thumbnail regardless — a missing `movie.nfo` degrades to
  filename-based Plex scraping, not a broken library entry.
- [Two separate YouTube API repositories now exist for videos
  (`YoutubeVideoRepository`, `YoutubeMetadataRepository`), both hitting
  the same endpoint shape] → Accepted duplication in exchange for keeping
  each caller's contract narrow; revisit only if a third caller emerges.
