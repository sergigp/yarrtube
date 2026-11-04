# Tasks

## 1. Walking skeleton

- [x] 1.1 Add every type and signature from design.md: `summary` on `VideoView`, `CategoryVideoView`, the two responses, `Video` and `aVideo`; `video_summary_repository` on `VideoSearcher::new`, passed from `src/serve.rs` and at the 74 calls in `src/application/http/videos/mod.rs`; the `view`, `category_view` and `source_summaries_enabled` signatures; drop `#[allow(dead_code)]` from `VideoSummaryRepository::find`. Bodies return `None`/`false`. Done when `cargo build` succeeds, `npm run typecheck` passes and all existing tests pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_include_the_summary_when_listing_playlist_videos`: a listed playlist video carries its stored summary.
- [x] 2.2 `it_should_hide_the_summary_when_ai_is_off`: AI off hides it, the row stays stored.
- [x] 2.3 `it_should_hide_the_summary_when_summaries_are_off`: summaries off hides it.
- [x] 2.4 `it_should_hide_the_summary_of_a_playlist_with_summaries_off`: the playlist's toggle hides it.
- [x] 2.5 `it_should_include_the_summary_when_listing_channel_videos`: a listed channel video carries its stored summary.
- [x] 2.6 `it_should_hide_the_summary_of_a_channel_with_summaries_off`: the channel's toggle hides it.
- [x] 2.7 `it_should_report_no_summary_for_a_video_without_one`: no stored summary reports `None`.
- [x] 2.8 `it_should_include_the_summary_when_listing_category_videos`: a category video carries its stored summary.
- [x] 2.9 `it_should_hide_the_summary_of_a_category_video_when_ai_is_off`: AI off hides it in a category.
- [x] 2.10 `it_should_hide_the_summary_of_a_category_video_whose_source_opted_out`: the source's toggle hides it in a category.
- [x] 2.11 `it_should_follow_the_channel_copy_of_a_category_video`: the channel's toggle decides when both copies exist.
- [x] 2.12 VideoDetail `shows the AI summary`: the region "AI summary" holds the exact text, line breaks kept with `whitespace-pre-line`.
- [x] 2.13 VideoDetail `shows no AI summary label when the video has none`.
- [x] 2.14 AiSettingsView `refetches video lists after saving`: a cached, off-screen channel video list is requested again after saving; `useSavePreferences` invalidates them with `refetchType: 'all'`.
- [x] 2.15 Change the `SummariesField` hint to "Generate and show an AI summary of each video." and update the `AddChannelDialog` and `AddPlaylistDialog` assertions; `npm run test` passes.
- [x] 2.16 Update `doc/AI.md` and `doc/ARCHITECTURE.md` to say summaries are shown and when they're hidden, and replace "stores it for later features to show" in the Purpose of `openspec/specs/video-summaries/spec.md`; verify by reading them against the specs.

## 3. Infrastructure adapters (TDD)

No adapter changes: `VideoSummaryRepository::find` exists and is tested.

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh --only web --only rust` passes.
- [x] 4.2 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, seed a playlist through the HTTP API, turn AI and summaries on and opt the playlist in. Insert a `video_summaries` row and set `summary_state` to `GENERATED` with `sqlite3`, since no Ollama is assumed. With the Playwright MCP browser check at desktop and mobile widths that the pane shows "AI summary" with the text. Turn the playlist's toggle off and check it disappears and the listing's `summary` is null; turn it on and check it returns; turn AI off in AI settings, go back and check it's gone, with the row still stored. Then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.3 Manual (human): with a real Ollama, check that a real summary reads well in the pane on desktop and phone.
