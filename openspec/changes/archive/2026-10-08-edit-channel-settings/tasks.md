## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures), wired end-to-end with trivial bodies:
  - Backend:
    - `Channel::with_quality` / `with_video_limit` returning `self` unchanged
    - `ChannelView.quality` / `video_limit` filled by `ChannelViewSearcher`
    - `ChannelRepository::update` on `SqliteChannelRepository` returning `Ok(())`
    - `UpdateChannelError`; `ChannelUpdater` + `ChannelUpdaterApi::update_settings` returning `NotFound`
    - DTOs: `UpdateChannelRequest`; `ChannelListItemResponse.quality` / `video_limit` (from the view)
    - `update_channel` handler returning the updater's result, `NOTHING_TO_UPDATE`, `PATCH /channels/{handle}` route, `ApiServices.channel_updater`, built in `serve.rs`
  - Existing `ChannelListItemResponse` expectations get the two new fields.
  - Web:
    - `ChannelListItem.quality` / `video_limit` + `aChannel` defaults
    - `UpdateChannelRequest`, `updateChannel` client, `useUpdateChannelSettings`
    - `lib/channelSettings.ts` returning `{}` / `false`
    - `EditChannelDialog` rendering an empty dialog, not placed in any view
    - `EntryActionsMenu` / `DetailHeader` `onEditRequest` prop, unused

  Done when `cargo build` succeeds and `cargo test --locked` and `npm run check` pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_update_a_channels_quality`: PATCH `{quality: "low"}` on a `high` channel → 200 with `low`, stored as `original.with_quality(Low)`
- [x] 2.2 `it_should_update_a_channels_video_limit`: PATCH `{video_limit: 20}` on a limit-5 channel → 200 with 20, stored as `original.with_video_limit(20)`
- [x] 2.3 `it_should_update_both_settings_of_a_channel`: both stored and returned
- [x] 2.4 `it_should_leave_a_channel_unchanged_if_settings_already_set`: 200, storage unchanged
- [x] 2.5 `it_should_fail_to_update_an_unknown_channel`: 404 `channel <handle> not found`, nothing stored
- [x] 2.6 `it_should_fail_to_update_if_nothing_to_update`: `{}` → 400 `NOTHING_TO_UPDATE`
- [x] 2.7 `it_should_fail_to_update_if_invalid_quality_provided`: 400 with the `Quality` validation message, storage unchanged
- [x] 2.8 `it_should_fail_to_update_if_invalid_video_limit_provided`: 1001 → 400 with the `VideoLimit` range message, storage unchanged
- [x] 2.9 `it_should_fail_to_update_if_invalid_handle_provided`: 400 with the `ChannelHandle` message
- [x] 2.10 `it_should_list_channels_with_their_quality_and_video_limit`: list response carries each channel's quality and limit
- [x] 2.11 `lib/channelSettings` unit tests: `channelSettingsChanges` returns only changed fields or `{}`; `lowersVideoLimit` true only below the current limit
- [x] 2.12 `EditChannelDialog` "opens prefilled with the channel's current settings"
- [x] 2.13 `EditChannelDialog` "sends only the changed quality"
- [x] 2.14 `EditChannelDialog` "sends nothing and closes when nothing changed"
- [x] 2.15 `EditChannelDialog` "warns when lowering the video limit" (and not when raising)
- [x] 2.16 `EditChannelDialog` "does not submit an out-of-range video limit"
- [x] 2.17 `EditChannelDialog` "keeps the dialog open and shows the error when saving fails"
- [x] 2.18 `ChannelDetail` "edits the channel's settings from its page header menu": "Edit settings" item, PATCH `{quality}` routed, no reconcile
- [x] 2.19 `ChannelDetail` "syncs the channel after its video limit changes": PATCH then POST reconcile
- [x] 2.20 `ChannelDetail` "alerts when the sync after saving fails"
- [x] 2.21 `PlaylistDetail` "offers no Edit settings item"

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `SqliteChannelRepository` `it_should_update_an_existing_channel`: `update` with new quality and limit → `find` returns them, other fields unchanged

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass
- [x] 4.2 `npm run check` passes in `web/`
- [x] 4.3 Extend `smoke-tests/tests/channel.spec.js`: change the channel's video quality through its page header "⋮" menu; reopening "Edit settings" shows the new value. Run `scripts/run-smoke-tests.sh` and it passes
  - 22/23 passed, including the channel lifecycle with the new step. The one failure, `mobile-sidebar.spec.js` "opens via the menu button and closes", is a pre-existing flake: run alone against an image built from `main` it failed 3 of 5 times the same way (the overlay intercepts the "Close menu" click).
- [x] 4.4 Manual check with `scripts/run-local.sh`: edit a channel's quality and limit; raising the limit syncs and downloads more; lowering shows the deletion warning
  - Done against the real binary on a scratch DB with a seeded channel (no `YOUTUBE_API_KEY` available, so `run-local.sh` couldn't add one): PATCH validation/404 via curl; UI "Edit settings" opens prefilled; lowering 10 → 3 shows the warning; saving stores it and the triggered sync listed exactly 3 videos.

## Workflow follow-up

- Archive the change (`/opsx:archive`) once implemented, then open the PR referencing #88.
