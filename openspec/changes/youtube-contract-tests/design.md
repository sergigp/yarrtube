## Files

- `src/infrastructure/shared/youtube_contract.rs` (new, `#[cfg(test)]`): what every contract test shares: the real `yt-dlp` path, the API key, the fixture ids with their credits, and `ffprobe` codec reading. Under `shared/` because the tests of several adapters use it.
- `src/infrastructure/shared/mod.rs`: declares it under `#[cfg(test)]`.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs`: `mod youtube_contract_tests` with the download, thumbnail, diagnosis, caption and subtitle tests, colocated with the adapter.
- `src/infrastructure/repositories/youtube_channel_videos_repository.rs`: `mod youtube_contract_tests`, the channel listing test.
- `src/infrastructure/repositories/youtube_playlist_repository.rs`, `youtube_playlist_items_repository.rs`, `youtube_metadata_repository.rs`, `youtube_channel_repository.rs`, `filesystem_channel_avatar_repository.rs`: `mod youtube_contract_tests` each, their Data API tests.
- `scripts/run-youtube-contract-tests.sh` (new): loads `.env`, checks `yt-dlp`, `ffprobe` and `YOUTUBE_API_KEY`, runs `cargo test --locked -- --ignored youtube_contract`.
- `doc/DEVELOPMENT.md`: how and when to run them.

Every test is `#[ignore = "real YouTube: run scripts/run-youtube-contract-tests.sh"]` inside a module named `youtube_contract_tests`, so the default `cargo test` skips them and the script's name filter selects exactly them.

## Types & Signatures

```rust
// src/infrastructure/shared/youtube_contract.rs  (#[cfg(test)])

/// "Boat in Island", 14 s, CC BY, Free HD videos - no copyright.
pub(crate) const SHORT_VIDEO_ID: &str = "mqWUW6ypo6w";
/// "Big Buck Bunny", CC BY, Blender Foundation (@BlenderOfficial): uploaded `en` and `fr` subtitles.
pub(crate) const CAPTIONED_VIDEO_ID: &str = "YE7VzlLtp-4";
/// An id YouTube has no video for.
pub(crate) const MISSING_VIDEO_ID: &str = "aaaaaaaaaaa";
/// The project's "test" playlist (maintainer-owned), holding only Big Buck Bunny.
pub(crate) const PLAYLIST_ID: &str = "PLXWRoRTUXjks";
/// "Free HD videos - no copyright", CC BY uploads, inactive since 2023.
pub(crate) const CHANNEL_HANDLE: &str = "@FreeHDvideosnocopyright";

pub(crate) fn video_url(id: &str) -> String;
/// `YTDLP_PATH`, else `yt-dlp` on `PATH`.
pub(crate) fn ytdlp_path() -> PathBuf;
/// `YOUTUBE_API_KEY`; panics with how to set it when missing.
pub(crate) fn api_key() -> String;
/// `(codec_type, codec_name)` of every stream, via `ffprobe`.
pub(crate) fn stream_codecs(media: &Path) -> Vec<(String, String)>;
```

No production type or signature changes.

## Call Stack

- Run: `scripts/run-youtube-contract-tests.sh` → `cargo test --locked -- --ignored youtube_contract` → each `#[ignore]`d `it_should_*` in a `youtube_contract_tests` module.
- yt-dlp test: test → `YoutubeVideoDownloaderRepository::new(ytdlp_path())` / `YoutubeChannelVideosRepository::new(ytdlp_path())` → port method (`download(url, Quality::Low, staging)`, `fetch_thumbnail`, `diagnose`, `probe_captions`, `fetch_subtitles`, `list_current_videos(handle, 2)`) → real `yt-dlp` → YouTube; staging dirs from `test_support::unique_temp_dir`, removed at the end.
- Data API test: test → `Youtube*Repository::new(api_key())` → port method (`resolve`, `list_current_videos`, `find`) → `https://www.googleapis.com/youtube/v3/...`.
- Avatar test: `YoutubeChannelRepository::resolve(handle)` → `avatar_url` → `FilesystemChannelAvatarRepository::new(dir).store(handle, url)` → yt3 image host.

## Test Plan

These assert existing adapters against real YouTube, so they are expected to pass on the first run: each is kept as a regression test. A failing one is a real contract difference: stop and report it. Asserts compare whole values; where YouTube varies a field run to run (an avatar URL's signature, the SABR notice, the long auto-caption language list), that field is taken from the result and checked separately.

1. Behaviour tests: none (no application-layer behaviour changes).
2. Infrastructure tests:
   - `YoutubeVideoDownloaderRepository`:
     1. `it_should_download_a_playable_mp4`: `Succeeded(DownloadedVideo { media_path: <staging>/mqWUW6ypo6w.mp4, thumbnail_path: Some(<staging>/mqWUW6ypo6w.jpg), duration_seconds: Some(14), published_at: Some(<its publish time>), .. })`, and `ffprobe` finds an `h264` video and an `aac` audio stream.
     2. `it_should_fail_to_download_a_missing_video`: `Failed { stderr: Some("ERROR: [youtube] aaaaaaaaaaa: This video is unavailable"), sabr_notice: None }`.
     3. `it_should_fetch_only_the_thumbnail`: `Fetched(FetchedThumbnail { path: <staging>/mqWUW6ypo6w.jpg, published_at })`, and the staging dir holds only that file.
     4. `it_should_diagnose_no_failure_for_a_playable_video`: `Ok(None)`.
     5. `it_should_diagnose_why_a_missing_video_failed`: `Ok(Some(<YouTube's reason>))`.
     6. `it_should_probe_the_caption_tracks`: `CaptionTracks { original_language, manual: ["en", "fr"], automatic }` (automatic: non-empty, includes `en`), its publish time, and the info JSON written to the staging dir.
     7. `it_should_fetch_the_uploaded_subtitles_as_srt`: `FetchedSubtitles { path: <staging>/YE7VzlLtp-4.en.srt, srt }` whose `srt` opens with cue `1` and a `-->` timing line.
   - `YoutubeChannelVideosRepository`:
     8. `it_should_list_the_newest_channel_videos`: the two newest uploads `[mUxzKVrSAjs, dj3Hfai1xEE]` with titles, positions `0, 1`.
   - `YoutubePlaylistRepository`:
     9. `it_should_resolve_a_playlist`: `Some(ResolvedPlaylist { title: "test", item_count: 1 })`.
     10. `it_should_not_resolve_an_unknown_playlist`: `Ok(None)`.
   - `YoutubePlaylistItemsRepository`:
     11. `it_should_list_the_playlist_videos`: `[YoutubePlaylistItem { video_id: "YE7VzlLtp-4", title: "Big Buck Bunny", position: 0 }]`.
   - `YoutubeMetadataRepository`:
     12. `it_should_find_a_video_metadata`: the whole `YoutubeMetadata` of `mqWUW6ypo6w`.
     13. `it_should_not_find_an_unknown_video`: `Ok(None)`.
   - `YoutubeChannelRepository`:
     14. `it_should_resolve_a_channel_handle`: `ResolvedChannel { youtube_channel_id: "UCJ0BmJOn_bRKiApjc45QZ1w", title: "Free HD videos - no copyright", avatar_url: Some(https://yt3.ggpht.com/…) }`.
     15. `it_should_not_resolve_an_unknown_handle`: `Ok(None)`.
   - `FilesystemChannelAvatarRepository`:
     16. `it_should_store_a_channel_avatar`: storing the resolved avatar URL returns `Some("@FreeHDvideosnocopyright.jpg")` and writes a non-empty image there.
