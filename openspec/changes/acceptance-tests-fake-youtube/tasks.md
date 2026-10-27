## 1. Walking skeleton

- [x] 1.1 Create every file and signature from design.md with trivial bodies: the two settings read in `serve.rs` and `youtube_api_url` wired through `InfrastructureSettings` to the four adapters (`new(api_key, api_url)`, still calling their current endpoints), `UpdateYtdlpTask::new(.., self_update_enabled, ..)` (ignored for now), `smoke-tests/fake-youtube/` (catalogue with the fixture world, a stub answering nothing, a stand-in that exits 1), `helpers/fakeYoutube.js`, `addPlaylist`/`addChannel` stubs, `fixture.en.srt`, and the container configuration in `run-smoke-tests.sh`. Done when `cargo build`, `cargo test --locked` and `npm --prefix web run check` pass and `npx --prefix smoke-tests playwright test --list` lists the specs.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_skip_the_update_and_reschedule_if_self_update_is_off`: `UpdateYtdlpTask` skips the updater when off; `serve.rs` skips the startup update when off.
- [x] 2.2 Smoke: the stub serves the fixture playlist and the stand-in downloads it: `playlist golden path` passes via `scripts/run-smoke-tests.sh`.
- [x] 2.3 Smoke: `playlist.spec.js` fully on the fixture playlist (seeded tests use `addPlaylist`); the file passes.
- [x] 2.4 Smoke: `channel.spec.js` and `channelVideoLimit.spec.js` on the fixture channel; both pass.
- [x] 2.5 Smoke: `moveSources.spec.js` and `categories.spec.js`; both pass.
- [x] 2.6 Smoke: the add-dialog specs (`addDialogLocation`, `addPlaylistDialog`, `addChannelDialog`, incl. unknown ids and the avatar in the notice); all pass.
- [ ] 2.7 Smoke: `deepLinks`, `deleteDuringDownload` (slow fixture video), `media` (Range only), `playbackSpeed`; all pass.
- [ ] 2.8 Smoke: the `@mobile` specs (`mobile-sidebar`, `mobileWatch`, `mobileLayout`); pass on `mobile-safari`.
- [ ] 2.9 Smoke: `a download that keeps failing is shown as errored with its reason` (`erroredDownload.spec.js`) passes.
- [ ] 2.10 Smoke: `sidebar search lists only matching playlists` and `tasks search lists only matching tasks` (`search.spec.js`) pass.
- [ ] 2.11 Smoke: `an upcoming premiere is neither listed nor downloaded` (`premiere.spec.js`) passes.

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `it_should_read_the_youtube_api_url_or_default_it` (`serve.rs`) passes.
- [x] 3.2 `it_should_turn_the_ytdlp_self_update_off_only_if_false` (`serve.rs`) passes.
- [x] 3.3 The four Data API adapters append their endpoint to the configured root: their mock-server tests use `new(api_key, server.url())` and the endpoint path, and pass; the contract tests (`scripts/run-youtube-contract-tests.sh`) still pass.

- [x] 3.4 `it_should_read_the_background_poll_interval_or_default_it` (`serve.rs`) passes; both background loops use it and the smoke container sets 250 ms.

## 4. Verification

- [ ] 4.1 `doc/INSTALLATION.md` (the three settings, for testing) and `smoke-tests/README.md` (the fake YouTube, its catalogue, what still needs the real one) updated; `scripts/run-tests.sh` passes, and `scripts/run-smoke-tests.sh` passes with no `YOUTUBE_API_KEY` (upgrade check skipped) and with one (upgrade check run). Record both run times.
