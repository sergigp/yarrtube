## 1. Walking skeleton

- [x] 1.1 Create `src/infrastructure/shared/youtube_contract.rs` (`#[cfg(test)]`, fixture constants with credits, `video_url`, `ytdlp_path`, `api_key`, `stream_codecs`) and `scripts/run-youtube-contract-tests.sh` (loads `.env`, checks `yt-dlp`, `ffprobe`, `YOUTUBE_API_KEY`, runs `cargo test --locked -- --ignored youtube_contract`). Done when `cargo build` and `cargo test --locked` pass, and the script runs and reports 0 tests.

## 2. Behaviour (TDD)

None: no application-layer behaviour changes (see design.md's Test Plan).

## 3. Infrastructure adapters (TDD)

Each is a real-YouTube test of an existing adapter, `#[ignore]`d in a `youtube_contract_tests` module and run with the script; expected to pass at once (kept as a regression test). A failure is a contract difference: stop and report it.

`YtDlpVideoDownloaderRepository`:
- [x] 3.1 `it_should_download_a_playable_mp4`: passes via the script.
- [x] 3.2 `it_should_fail_to_download_a_missing_video`: passes via the script.
- [x] 3.3 `it_should_fetch_only_the_thumbnail`: passes via the script.
- [x] 3.4 `it_should_diagnose_no_failure_for_a_playable_video`: passes via the script.
- [x] 3.5 `it_should_diagnose_why_a_missing_video_failed`: passes via the script.
- [x] 3.6 `it_should_probe_the_caption_tracks`: passes via the script.
- [x] 3.7 `it_should_fetch_the_uploaded_subtitles_as_srt`: passes via the script.

`YtDlpChannelVideosRepository`:
- [x] 3.8 `it_should_list_the_newest_channel_videos`: passes via the script.

`YoutubeApiPlaylistRepository`:
- [ ] 3.9 `it_should_resolve_a_playlist`: passes via the script.
- [ ] 3.10 `it_should_not_resolve_an_unknown_playlist`: passes via the script.

`YoutubeApiPlaylistItemsRepository`:
- [ ] 3.11 `it_should_list_the_playlist_videos`: passes via the script.

`YoutubeApiMetadataRepository`:
- [ ] 3.12 `it_should_find_a_video_metadata`: passes via the script.
- [ ] 3.13 `it_should_not_find_an_unknown_video`: passes via the script.

`YoutubeApiChannelRepository`:
- [ ] 3.14 `it_should_resolve_a_channel_handle`: passes via the script.
- [ ] 3.15 `it_should_not_resolve_an_unknown_handle`: passes via the script.

`FilesystemChannelAvatarRepository`:
- [ ] 3.16 `it_should_store_a_channel_avatar`: passes via the script.

## 4. Verification

- [ ] 4.1 Document the contract tests in `doc/DEVELOPMENT.md` (what they check, prerequisites, the command, that CI skips them); `cargo test --locked` (contract tests reported as ignored), `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass, and `scripts/run-youtube-contract-tests.sh` passes all 16. No smoke run needed: no production code changes.
