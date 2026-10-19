## Files

- `src/domain/video/web_vtt.rs`: `WebVtt` value. Built from SRT text, it adds the `WEBVTT` header, converts `,` to `.` in timings, strips any BOM and normalises CRLF.
- `src/domain/video/video.rs`: adds `subtitle_language()`, derived from `subtitle_filename` when the state is Fetched.
- `src/domain/video/errors.rs`: adds `ReadSubtitlesError`.
- `src/domain/video/mod.rs`: re-exports `WebVtt`.
- `src/domain/services/subtitle_reader.rs`: the use case "read a source video's subtitles as WebVTT".
- `src/domain/services/mod.rs`: registers and re-exports `SubtitleReader`.
- `src/infrastructure/repositories/filesystem_video_file_repository.rs`: adds a `read_to_string` port method, the real implementation, and a fake with `with_file` seeding.
- `src/application/http/videos/mod.rs`: the `get_playlist_video_subtitles` and `get_channel_video_subtitles` handlers, plus `read_subtitles_error`.
- `src/application/http/videos/dto.rs`: adds the `VideoResponse.subtitle_language` field.
- `src/application/http/mod.rs`: adds `subtitle_reader` to `ApiServices` and the two routes.
- `src/serve.rs`: constructs `SubtitleReader` in the composition root.
- `web/src/api/types.ts`: adds `Video.subtitle_language`.
- `web/src/test/helpers.tsx`: the `aVideo` default becomes `subtitle_language: null`.
- `web/src/api/client.ts`: adds `videoSubtitlesUrl`.
- `web/src/lib/languageName.ts` (+ `.test.ts`): turns a code into an English name, falling back to the code.
- `web/src/components/VideoPlayer.tsx` (+ `.test.tsx`): renders `<track>` and takes a `source` prop.
- `web/src/components/PlaylistDetail.tsx`, `ChannelDetail.tsx`: pass `source` to the player.
- `smoke-tests/tests/playlist.spec.js`, `channel.spec.js`: request the subtitles route so route coverage stays green.

## Types & Signatures

```rust
// domain/video/web_vtt.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebVtt(String);
impl WebVtt {
    pub fn from_srt(srt: &str) -> Self;
    pub fn as_str(&self) -> &str;
}

// domain/video/video.rs
impl Video {
    /// `en` for `… .en.srt`; None unless subtitles are Fetched.
    pub fn subtitle_language(&self) -> Option<String>;
}

// domain/video/errors.rs
#[derive(Debug)]
pub enum ReadSubtitlesError {
    PlaylistNotFound(PlaylistId),
    ChannelNotFound(ChannelHandle),
    VideoNotFound(VideoId),
    SubtitlesNotFound(VideoId),
    Repository(anyhow::Error),
}

// domain/services/subtitle_reader.rs
#[derive(Clone)]
pub struct SubtitleReader {
    playlist_repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    videos_path: String,
}
pub trait SubtitleReaderApi: Send + Sync {
    fn read_for_playlist(&self, playlist_id: &PlaylistId, video_id: &VideoId)
        -> Result<WebVtt, ReadSubtitlesError>;
    fn read_for_channel(&self, channel_id: &ChannelHandle, video_id: &VideoId)
        -> Result<WebVtt, ReadSubtitlesError>;
}

// infrastructure/repositories/filesystem_video_file_repository.rs
pub trait VideoFileRepository {
    // …existing
    /// The file's UTF-8 content, None when it doesn't exist.
    fn read_to_string(&self, output_dir: &Path, filename: &str) -> anyhow::Result<Option<String>>;
}
#[cfg(test)] impl FakeVideoFileRepository { pub fn with_file(self, path: impl Into<PathBuf>, content: &str) -> Self; }

// application/http/videos/dto.rs
pub struct VideoResponse { /* …existing */ pub subtitle_language: Option<String> }

// application/http/videos/mod.rs
pub type VttResponse = ([(HeaderName, &'static str); 1], String);
pub async fn get_playlist_video_subtitles(
    State(subtitle_reader): State<SubtitleReader>,
    Path((playlist_id, video_id)): Path<(String, String)>,
) -> Result<VttResponse, ApiError>;
pub async fn get_channel_video_subtitles(
    State(subtitle_reader): State<SubtitleReader>,
    Path((handle, video_id)): Path<(String, String)>,
) -> Result<VttResponse, ApiError>;
fn read_subtitles_error(error: ReadSubtitlesError) -> ApiError;
```

```ts
// web
interface Video { /* … */ subtitle_language: string | null }
export type VideoSourceRef = { kind: 'playlist' | 'channel'; id: string }
export function videoSubtitlesUrl(source: VideoSourceRef, videoId: string): string
export function languageName(code: string): string
interface VideoPlayerProps { /* … */ source: VideoSourceRef }
```

## Call Stack

**GET subtitles (playlist; the channel route mirrors it)**
1. `get_playlist_video_subtitles(State(reader), Path((id, video_id)))` builds `PlaylistId::new(id)?` and `VideoId::new(video_id)?`, so invalid input is a 400.
2. `run_blocking(|| reader.read_for_playlist(&playlist_id, &video_id))`
   1. `playlist_repository.find(&playlist_id)`; None gives `PlaylistNotFound`.
   2. `video_repository.find_by_youtube_id(&video_id)`, then keep the video for which `playlist_video_repository.find_by_video(&video.id)` returns this `playlist_id`. None gives `VideoNotFound`.
   3. `video.subtitle_filename`, only when `subtitle_state == Fetched`; otherwise `SubtitlesNotFound`.
   4. `video_file_repository.read_to_string(&Path::new(&videos_path).join(playlist.path.as_str()), &subtitle_filename)`; None gives `SubtitlesNotFound`.
   5. `Ok(WebVtt::from_srt(&srt))`
3. `.map_err(read_subtitles_error)`: every `*NotFound` maps to 404 and `Repository` to 500.
4. `Ok(([(CONTENT_TYPE, "text/vtt; charset=utf-8")], vtt.as_str().to_string()))`

Notes (keep, don't "fix"):
- The `.vtt` is converted from the `.srt` on every request, not written as a sidecar. That means no backfill, no changes to cleanup or migrate-layout, and no duplicate tracks in Jellyfin/Kodi.
- Every not-found case is a 404, including an unknown source, because the URL names a resource. The list endpoints answer 400 for an unknown source.
- `subtitle_language` is parsed from the recorded filename, not the transcript row. That saves a query per listed video and always matches the file served.
- `HomeVideoResponse` is unchanged, because home cards don't play.

**List videos**: `VideoResponse::from(view)` sets `subtitle_language: video.subtitle_language()`.

**Web player**: `PlaylistDetail` passes `source={{kind:'playlist', id}}` and `ChannelDetail` passes `{kind:'channel', id: handle}` to `VideoPlayer`. When `video.subtitle_language` is set, the player renders `<track kind="subtitles" src={videoSubtitlesUrl(source, video.id)} srcLang={lang} label={languageName(lang)} />` without `default`.

## Test Plan

### 1. Behaviour
HTTP handlers (`src/application/http/videos/mod.rs`, real SQLite + `FakeVideoFileRepository`):
1. `it_should_serve_a_playlist_videos_subtitles_as_web_vtt`: a Fetched video with seeded `.srt` content returns `Ok((content-type text/vtt, "WEBVTT\n\n1\n00:00:01.500 --> 00:00:03.000\nHello there\n…"))`.
2. `it_should_serve_a_channel_videos_subtitles_as_web_vtt`: the same flow through the channel handler.
3. `it_should_fail_to_serve_subtitles_if_not_fetched`: a Missing state returns `Err(ApiError 404)`.
4. `it_should_fail_to_serve_subtitles_if_file_missing`: a Fetched state with no file returns 404.
5. `it_should_fail_to_serve_subtitles_if_video_not_in_source`: a video recorded for another playlist returns 404.
6. `it_should_fail_to_serve_subtitles_if_playlist_not_found`: 404.
7. `it_should_fail_to_serve_subtitles_if_channel_not_found`: 404.
8. `it_should_fail_to_serve_subtitles_if_invalid_video_id_provided`: 400 on an unmigrated DB.
9. `it_should_include_the_subtitle_language_when_listing_videos`: the playlist list returns `subtitle_language: Some("en")` for Fetched and `None` for Missing.

Edge cases unit-tested where they live (`web_vtt.rs`):
10. `it_should_convert_srt_timings_to_web_vtt`: header plus `,`→`.` in timing lines only; cue text keeps any commas.
11. `it_should_strip_a_bom_and_normalise_crlf`.

`video.rs`:
12. `it_should_report_the_subtitle_language_from_its_filename`: `.en.srt` gives `Some("en")`; Missing/Unavailable give `None`.

Web (Vitest):
13. `languageName`: `en` gives "English", `es` gives "Spanish", and an unknown code like `zz` returns `zz`.
14. `VideoPlayer`: with `subtitle_language: 'en'` it renders a track with `src=/api/playlists/p1/videos/v1/subtitles.vtt`, `srclang=en`, `label=English` and no `default`. With `null` it renders no track.

Smoke: in `playlist.spec.js` and `channel.spec.js`, wait until the list reports `subtitle_language` for the downloaded video. Then turn the track on in the page (`video.textTracks[0].mode = 'showing'`) and assert the browser's `subtitles.vtt` response is 200 `text/vtt` starting with `WEBVTT`. The route coverage reporter only records browser-hit routes. If the smoke videos turn out to have no captions, stop and ask before changing fixtures.

### 2. Infrastructure
`filesystem_video_file_repository.rs` (real temp dir):
1. `it_should_read_a_files_content`
2. `it_should_read_none_when_the_file_is_missing`
