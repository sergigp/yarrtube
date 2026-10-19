## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures) across all layers:
  - Domain: `WebVtt` (`from_srt` returns `WebVtt(String::new())`), `Video::subtitle_language` (returns `None`), `ReadSubtitlesError`, and `SubtitleReader` + `SubtitleReaderApi` (both reads return `Err(SubtitlesNotFound)`).
  - Infrastructure: `VideoFileRepository::read_to_string`, with the filesystem impl returning `Ok(None)` and the fake getting `with_file`.
  - HTTP: the `get_playlist_video_subtitles` / `get_channel_video_subtitles` handlers + `read_subtitles_error`, `VideoResponse.subtitle_language`, `ApiServices.subtitle_reader`, and both routes registered in `api_router`.
  - Wiring: `SubtitleReader` constructed in `serve.rs`.
  - Web: `Video.subtitle_language` in `types.ts`, a `subtitle_language: null` default in `aVideo`, `videoSubtitlesUrl` + `VideoSourceRef` in `client.ts`, `languageName` returning the code as-is, and a `source` prop on `VideoPlayer` passed from `PlaylistDetail` / `ChannelDetail`.

  Done when `cargo build`, `cargo test --locked` and `cd web && npm run check` all pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_serve_a_playlist_videos_subtitles_as_web_vtt`: a Fetched playlist video with seeded `.srt` returns `text/vtt` with the converted body. Drives `read_for_playlist`, `WebVtt::from_srt` and the fake's `read_to_string`.
- [x] 2.2 `it_should_serve_a_channel_videos_subtitles_as_web_vtt`: drives `read_for_channel`.
- [x] 2.3 `it_should_fail_to_serve_subtitles_if_not_fetched`: Missing state gives 404.
- [x] 2.4 `it_should_fail_to_serve_subtitles_if_file_missing`: Fetched but no file gives 404.
- [x] 2.5 `it_should_fail_to_serve_subtitles_if_video_not_in_source`: a video recorded only for another playlist gives 404.
- [x] 2.6 `it_should_fail_to_serve_subtitles_if_playlist_not_found`: 404.
- [x] 2.7 `it_should_fail_to_serve_subtitles_if_channel_not_found`: 404.
- [x] 2.8 `it_should_fail_to_serve_subtitles_if_invalid_video_id_provided`: 400, no DB needed.
- [x] 2.9 `it_should_include_the_subtitle_language_when_listing_videos`: `Some("en")` for Fetched `.en.srt`, `None` for Missing. Drives `Video::subtitle_language` and the DTO mapping.
- [x] 2.10 `web_vtt.rs` `it_should_convert_srt_timings_to_web_vtt`: only timing lines change `,`→`.`; commas in cue text are kept.
- [x] 2.11 `web_vtt.rs` `it_should_strip_a_bom_and_normalise_crlf`.
- [x] 2.12 `video.rs` `it_should_report_the_subtitle_language_from_its_filename`: Fetched gives the code; Missing/Unavailable give `None`.
- [x] 2.13 `web/src/lib/languageName.test.ts`: `en` gives "English", `es` gives "Spanish", unknown `zz` gives `zz` (via `Intl.DisplayNames`).
- [x] 2.14 `web/src/components/VideoPlayer.test.tsx`: with `subtitle_language: 'en'` it renders a `track` with the subtitles URL, `srclang="en"`, `label="English"` and no `default`; with `null` it renders no track. Remove the `media-has-caption` eslint-disable where a track is rendered. Verify with `npm run check`.

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `filesystem_video_file_repository.rs` `it_should_read_a_files_content`: the real temp dir returns `Some(content)`.
- [x] 3.2 `filesystem_video_file_repository.rs` `it_should_read_none_when_the_file_is_missing`: returns `Ok(None)`.

## 4. Verification

- [x] 4.1 Run `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `cd web && npm run check`. All must pass.
- [x] 4.2 Smoke: in `smoke-tests/tests/playlist.spec.js` and `channel.spec.js`, wait for `subtitle_language` on the downloaded video, turn the player's track on in the page, and assert the browser's `subtitles.vtt` response is 200 `text/vtt` starting with `WEBVTT`. Both new routes must be recorded by route coverage. Run `scripts/run-smoke-tests.sh`. If the smoke videos have no captions, stop and ask.
- [x] 4.3 Manual: run `scripts/run-local.sh` and open a video with Fetched subtitles. Confirm CC is off at start. Enable it from the player's captions menu, confirm cues show in sync, and confirm `subtitles.vtt` is requested only after enabling.
