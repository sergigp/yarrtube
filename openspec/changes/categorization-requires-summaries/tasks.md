# Tasks

## 1. Walking skeleton

- [x] 1.1 Add `VideoOwner::categorizes()` (returns `self.categorization_enabled` for now), `web/src/lib/sourceAi.ts` `sourceCategorizes` (returns `source.categorization_enabled`), and the optional `blockedBy` prop on `SourceAiSection`'s `AiCheckbox` (unused). Route `VideoReconciler::categorization_wanted`, `VideoCategorizer::prepare`, `VideoCategoryChoiceSearcher` and `ChannelDetail`/`PlaylistDetail` through them. Done when `cargo build` passes, `scripts/run-tests.sh --lint --web --rust` passes, and behaviour is unchanged.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_not_schedule_a_categorization_when_the_source_has_summaries_off` (sync_video_task): `VideoOwner::categorizes()` requires `summaries_enabled`.
- [x] 2.2 `it_should_not_schedule_a_categorization_when_ai_summaries_are_off` (sync_video_task): `Preferences::categorization_available()` requires summaries on.
- [x] 2.3 `it_should_skip_the_categorization_when_ai_summaries_are_off` (categorize_video_task): a queued run is skipped without calling the model.
- [x] 2.4 `it_should_skip_the_categorization_when_the_source_has_summaries_off` (categorize_video_task): the skip reason names summaries or categorization off.
- [x] 2.5 `it_should_fail_to_enable_categorization_on_a_channel_while_summaries_are_off` (http/channels): 409, nothing changed.
- [x] 2.6 `it_should_enable_categorization_on_a_playlist_with_its_summaries_off` (http/playlists): accepted and stored, inactive.
- [ ] 2.7 `it_should_report_categorization_off_for_a_video_whose_source_has_summaries_off` (http/videos): `categorization_enabled` is the effective value.
- [ ] 2.8 Web `preferences.test.ts`: `categorizationAvailable` requires `summaries_enabled`.
- [ ] 2.9 Web `AiSettingsView.test.tsx`: with summaries off, the Categorization switch is disabled, "Needs summaries." is shown and Model is disabled.
- [ ] 2.10 Web `AiSettingsView.test.tsx`: turning Summaries off unsaved keeps Categorization checked but disabled and hides Advanced/Prompt.
- [ ] 2.11 Web `AddChannelDialog.test.tsx`: "Categorize videos" is blocked with "Needs Summarize videos." until "Summarize videos" is checked; both are sent true.
- [ ] 2.12 Web `EditPlaylistDialog.test.tsx`: a playlist with summaries off has "Categorize videos" disabled with "Needs Summarize videos." until "Summarize videos" is checked.
- [ ] 2.13 Web `EditChannelDialog.test.tsx`: unchecking "Summarize videos" keeps "Categorize videos" checked and disabled, and only `summaries_enabled` `false` is sent.
- [ ] 2.14 Web `ChannelDetail.test.tsx`: a channel with categorization on and summaries off offers "Clear AI categories".
- [ ] 2.15 Docs: `doc/CATEGORIES.md` and `doc/AI.md` say categorization needs summaries on the AI page and on each channel or playlist, and that turning summaries off pauses categorization without clearing it.

## 3. Infrastructure adapters (TDD)

None. No adapter changes.

## 4. Verification

- [ ] 4.1 Run `scripts/run-tests.sh --lint --web --rust`; all pass.
- [ ] 4.2 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`. Seed AI on, categories on, summaries off and categorization on via `PATCH /api/preferences`, plus a channel via the API. With the Playwright MCP browser:
  - On `/settings/ai`, Categorization is disabled with "Needs summaries."; turning Summaries on enables it.
  - In the channel's "Edit settings", "Categorize videos" is disabled with "Needs Summarize videos." until "Summarize videos" is checked.
  - The channel menu offers "Clear AI categories" while the channel's summaries are off.

  Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
