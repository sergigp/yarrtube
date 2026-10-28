# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every backend file, type and signature from design.md: migration 0013 registered (adds `color` with default `#fff1bf`, no backfill yet), `CategoryColor` (`new` accepts anything, `least_used` returns the first palette color), the `color` field on `Category`/`CategoryView`, `CategoryCreator::create(name, color)`, `CategoryUpdater::update(id, CategoryUpdate)` replacing `rename` (applies the name only), `HomeVideoView::category_ids` (always empty), `VideoSearcher::with_categories` (returns views unchanged), the repository reading and writing `color`, and the DTO fields (`color`, `category_ids`). Existing tests updated for the new signatures only. Done when `cargo build` succeeds and `cargo test --locked` passes; no new tests.
- [x] 1.2 Create every web file, type and signature from design.md: `color`/`category_ids` on the types and fixture builders, `createCategory(name, color)`, `updateCategory(id, { name?, color? })` replacing `renameCategory`, `lib/categoryColors.ts` (palette, stub functions), `CategoryColorPicker` (placeholder), the `DetailHeader` `tint` prop and color slot, the `VideoGrid` `categoryColors` prop (ignored). Nothing new is shown yet. Done when `npm run check` passes in `web/`; no new tests.

## 2. Behaviour (TDD)

Backend acceptance tests (HTTP handlers):

- [x] 2.1 `it_should_create_a_category_with_a_color` — 201 with the given color, lowercased; verify `cargo test categories` passes
- [x] 2.2 `it_should_fail_to_create_if_color_invalid` — `"#fff"`, `"yellow"`, `"#gggggg"` give 400 and nothing is stored
- [x] 2.3 `it_should_create_the_first_category_with_the_first_palette_color` — no categories, no color → `#fff1bf`
- [x] 2.4 `it_should_create_a_category_with_the_least_used_palette_color` — Butter and Coral used → `#ffe0c7`; every color used once and Butter twice → `#ffe0c7`
- [x] 2.5 `it_should_list_categories_with_their_color` — the list item carries `color`
- [x] 2.6 `it_should_recolor_a_category` — PATCH with the color only gives 204, and the name and members are kept
- [x] 2.7 `it_should_rename_and_recolor_a_category` — both fields are applied
- [x] 2.8 `it_should_fail_to_update_without_name_or_color` — 400, category unchanged (replaces `it_should_fail_to_rename_if_name_missing`)
- [x] 2.9 `it_should_fail_to_update_if_color_invalid` — 400, category unchanged
- [x] 2.10 `it_should_not_recolor_if_name_taken` — 409, color unchanged
- [x] 2.11 `it_should_list_a_home_video_with_its_category` — source in "Chess" → `category_ids` = [Chess]; verify `cargo test videos` passes
- [x] 2.12 `it_should_list_a_home_video_without_categories` — `[]`
- [x] 2.13 `it_should_order_home_video_categories_by_name` — "Streams" and "chess" → [chess, Streams]
- [x] 2.14 `it_should_include_categories_reached_through_another_source` — channel copy shown, playlist excluded from home in "Highlights" → [Chess, Highlights]
- [x] 2.15 `it_should_list_a_category_reached_by_two_members_once`
- [x] 2.16 `it_should_list_category_videos_with_their_categories` — `GET /api/categories/:id/videos` includes "Chess" and "Streams"

Value object (`category_color.rs`):

- [x] 2.17 `CategoryColor` › accepts `#rrggbb` and lowercases it
- [x] 2.18 `CategoryColor` › rejects short, named and non-hex values

Web (Vitest):

- [x] 2.19 `lib/categoryColors` › `defaultColor` picks the least used, the earliest on a tie; verify `npm run test` passes
- [x] 2.20 `lib/categoryColors` › `cardBackground` for 0 (none), 1 (flat), 2 and 3 (135deg hard-stop bands in order) and 4+ (`#f2f2f2`)
- [x] 2.21 `lib/categoryColors` › `pastelFromHue` gives a soft hex and `hueOf` round-trips it within 1 degree
- [x] 2.22 `CategoryColorPicker` › choosing a swatch checks it and reports its hex
- [x] 2.23 `CategoryColorPicker` › a non-palette color shows "Custom" checked, and the slider reports pastel colors
- [ ] 2.24 `AddCategoryDialog` › preselects the next free color and creates with the chosen one
- [ ] 2.25 `CategoryDetail` › tints the header with the category color and leaves the cards untinted
- [ ] 2.26 `CategoryDetail` › "Change color" saves the chosen color and the header updates
- [ ] 2.27 `SidebarCategories` › shows a dot in each category's color
- [ ] 2.28 `Home` › tints a one-category card, splits a two-category card diagonally, makes a four-category card grey and leaves an uncategorized card untinted
- [ ] 2.29 `CategoryDetail` (rename) › renaming still works through `updateCategory`; verify existing rename tests pass against the new route body

## 3. Infrastructure adapters (TDD)

SqliteCategoryRepository:

- [x] 3.1 `it_should_return_the_color_after_inserting_a_category`; verify `cargo test sqlite_category_repository` passes
- [x] 3.2 `it_should_update_the_color`

sqlite_migrations:

- [x] 3.3 `it_should_backfill_existing_categories_with_palette_colors_in_creation_order` — three pre-0013 categories → `#fff1bf`, `#ffe0c7`, `#ffd9d4`, wrapping after ten; verify `cargo test sqlite_migrations` passes

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass
- [ ] 4.2 `npm run check` in `web/` passes
- [ ] 4.3 `scripts/run-tests.sh` passes (the e2e categories specs still pass with the new update body)
- [ ] 4.4 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, create 3 categories through the API (one without a color, one with a custom hex), put one channel in 1 category and another in 2, then use the Playwright MCP browser to check: the sidebar dots, a flat-tinted card and a diagonally split card on home (screenshot), the tinted header and untinted grid on a category view, the Add category dialog preselecting the next free color, and "Change color" updating the header and the sidebar dot. Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.5 Manual (human): judge the 10 swatches and the custom slider by eye against real thumbnails, and confirm all of them feel soft and tell each other apart (Aqua vs Sky, Pistachio vs the active-row green, Sand vs the grey for 4+ categories); tweak the hexes if needed.

## Workflow follow-up

- Archive the change so `openspec/specs/category-crud`, `video-listing` and `web-ui` are updated, then verify with `openspec validate --specs`.
