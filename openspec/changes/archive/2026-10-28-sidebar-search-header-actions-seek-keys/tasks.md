# Tasks

Implemented on the `frontend-tweaks` branch before this change was written; boxes record what is already done and verified.

## 1. Walking skeleton

- [x] 1.1 Add the files and signatures from design.md (`seekKeys.ts`, `useSeekKeys.ts`, `HeaderAction`/`extraActions`, generic `sectionView`, `searchText` prop, `test/viewport.ts`) and verify `npm run check` passes

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_filter_categories_and_hide_section_without_match` — sidebar search filters categories; verify `SidebarCategories.test.tsx` passes
- [x] 2.2 `it_should_show_matches_of_a_folded_section_while_searching` — folded Categories shows matches; verify `SidebarCategories.test.tsx` passes
- [x] 2.3 `it_should_put_search_above_categories_and_count_them_toward_threshold`; verify `Sidebar.test.tsx` passes
- [x] 2.4 `it_should_keep_every_action_including_sync_behind_the_menu_on_mobile`; verify `DetailHeader.test.tsx` passes
- [x] 2.5 `it_should_lay_actions_out_as_icon_buttons_without_menu_on_desktop`; verify `DetailHeader.test.tsx` passes
- [x] 2.6 `it_should_name_an_icon_button_in_a_tooltip_on_hover`; verify `DetailHeader.test.tsx` passes
- [x] 2.7 `it_should_run_sync_and_alert_on_failure_on_desktop`; verify `DetailHeader.test.tsx` passes
- [x] 2.8 `it_should_offer_every_action_in_the_menu_on_mobile / as_buttons_on_desktop`; verify `PlaylistDetail.test.tsx` and `ChannelDetail.test.tsx` pass
- [x] 2.9 `it_should_read_arrows_as_directions_and_ignore_modified_fields_and_handled_keys`; verify `seekKeys.test.ts` passes
- [x] 2.10 `it_should_seek_ten_seconds_within_the_video`; verify `seekKeys.test.ts` passes
- [x] 2.11 `it_should_seek_ten_seconds_on_arrow_keys`; verify `VideoPlayer.test.tsx` passes
- [x] 2.12 `it_should_add_up_quick_presses_and_fade_out`; verify `VideoPlayer.test.tsx` passes
- [x] 2.13 `it_should_leave_arrows_typed_into_a_field_alone`; verify `VideoPlayer.test.tsx` passes

## 3. Infrastructure adapters (TDD)

No adapter changes (web UI only).

## 4. Verification

- [x] 4.1 `npm run check` in `web/` (typecheck, lint, 442 tests) passes
- [x] 4.2 `E2E_UPGRADE_FROM_IMAGE=none scripts/run-tests.sh --only e2e` passes (68/68, desktop Chromium + iPhone WebKit), including the new `search`, `detailHeader`, `seekKeys` specs and the updated `mobileLayout` Sync step
- [x] 4.3 Manual (agent): screenshot of a playlist page header on desktop shows square icon buttons with the "Edit settings" tooltip on hover
- [ ] 4.4 Manual (human): on a real phone, the header shows only the "⋮" and Sync works from it; on desktop, ←/→ seek feels right and the overlay looks right in light and dark themes

## Workflow follow-up

- Archive the change so `openspec/specs/web-ui` and `openspec/specs/e2e-tests` are updated, then verify with `openspec validate --specs`.
