## 1. Walking skeleton

- [x] 1.1 Create the backend parts of design.md's Files and Types & Signatures:
  - Add `list` to `ChannelVideoRepository`. The `SqliteChannelVideoRepository` body returns `Ok(Vec::new())` for now.
  - Leave `ChannelViewSearcher` on its per-video count until task 3.2, so the existing count test keeps passing.
  - Done when `cargo build` succeeds and all existing tests pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_list_channels_sorted_by_name_ignoring_case`: `GET /api/channels` returns channels by name, case-insensitive, regardless of creation order (`ORDER BY name COLLATE NOCASE, rowid`).
- [ ] 2.2 `it_should_list_playlists_sorted_by_name_ignoring_case`: `GET /api/playlists` returns playlists by name, case-insensitive, regardless of creation order.

## 3. Infrastructure adapters (TDD)

**`SqliteChannelVideoRepository`**
- [ ] 3.1 `it_should_list_every_channel_video`: `list` returns every stored `ChannelVideo` across all channels in one query.
- [ ] 3.2 Refactor `ChannelViewSearcher` to count from 3 bulk reads, as in design.md's Call Stack: `ChannelRepository::list`, `ChannelVideoRepository::list`, then `VideoRepository::find_many`. Fold into `HashMap<ChannelHandle, usize>` in Rust (missing entry = 0) and remove the per-video `count_unwatched`. Done when `it_should_count_unwatched_downloaded_videos_when_listing_channels`, `it_should_list_all_channels` and the full suite pass unchanged.

## 4. Verification

- [ ] 4.1 Add `@tanstack/react-query` and wrap `App` in a `QueryClientProvider` in `main.jsx`. Create `web/src/queries.js` as in design.md. Replace every `usePolling` call (Sidebar, ChannelDetail, PlaylistDetail, Home, TasksView) with its query hook, then delete `usePolling.js`. Verify with `npm run build` and `npm run lint` in `web/`.
- [ ] 4.2 Call `useInvalidateLibrary` after sync, mark watched and delete (sidebar rows and detail headers) and after a successful add (`AddDialog`). In `useWatchProgress`, invalidate `queryKeys.channels` after each successful non-beacon progress report. Verify with `npm run build` and `npm run lint`.
- [ ] 4.3 Create `web/src/sidebarSections.js` (`orderChannels`, `collapse`, `channelLeadCount`, `matchesSearch` and the thresholds 10 / 5 / 15). In `Sidebar.jsx` add:
  - collapsed sections with a "Show N more" / "Show less" control
  - the active entry always visible
  - expanded state per section in localStorage (`yarrtube.sidebar.expanded.<section>`, read and written in try/catch)
  - the shared search field, shown when channels + playlists > 15, which ignores collapse, hides sections with no matches and shows a "nothing matches" message when both are empty

  Verify with `npm run build` and `npm run lint`.
- [ ] 4.4 Check that the smoke tests' sidebar lookups (`helpers/sidebar.js`, `channel.spec.js`, `playlist.spec.js`) still find their rows under the new ordering and collapse. Verify with `scripts/run-smoke-tests.sh`, where all specs pass.
- [ ] 4.5 Run `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings`. All must pass.
- [ ] 4.6 Manual check with `scripts/run-local.sh`, at desktop width and at a mobile width of about 375px:
  - Unread channels come first, most unread first, and the rest are alphabetical.
  - A channel with 10 or more unwatched is ordered above one with 9.
  - With no unread channels, 5 are shown and then "Show N more".
  - Opening a collapsed channel from search keeps it visible and active after the search is cleared.
  - Expanded state survives a reload.
  - The search field appears only above 15 entries and filters both sections.
  - Marking a channel watched moves it into the caught-up group at once.
  - In the network tab, `/api/channels` polls every ~60s, stops while the tab is hidden and refetches when you return.
