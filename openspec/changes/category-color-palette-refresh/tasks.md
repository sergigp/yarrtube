# Tasks

## 1. Walking skeleton

- [x] 1.1 Create `migrations/0019_category_palette_refresh.sql` as a no-op statement (`SELECT 1;`), add `CATEGORY_PALETTE_REFRESH_SQL` and register it after 0018 in `sqlite_migrations::apply`. Add a `customChosen` state to `CategoryColorPicker` that nothing uses yet. Done when `cargo build`, `cargo test --locked` and `npm run check` (in `web/`) pass with no behaviour change.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_create_the_first_category_with_the_first_palette_color`: expect `#f59e0b`; switch `CategoryColor::PALETTE` to the new hexes. Verify with `cargo test categories`.
- [x] 2.2 `it_should_create_a_category_with_the_least_used_palette_color`: used `["#f59e0b", "#ef4444"]` and every color + amber again → `#f97316` both times. Verify with `cargo test categories`.
- [x] 2.3 `categoryColors.test.ts`: new palette names and hexes, new default-color expectations, `solidFromHue` at OKLCH 0.65 / 0.2 keeps hue within 3°, soft tint and ink contrast hold for every new color. Update `PALETTE` and rename `SOLID` → `CUSTOM` in `categoryColors.ts`. Update the fixture color in `test/helpers.tsx`. Verify with `npm run test -- categoryColors`.
- [ ] 2.4 `CategoryColorPicker.test.tsx`: swatches Amber … Slate then Custom; Sky reports `#0ea5e9`; the existing custom-hue tests use the new hexes. Verify with `npm run test -- CategoryColorPicker`.
- [ ] 2.5 `CategoryColorPicker.test.tsx`: choosing Custom from Orange shows Custom checked and the slider. Choosing it from Blue does the same. Make the picker derive `custom` from `customChosen || !inPalette(value)`. Verify with `npm run test -- CategoryColorPicker`.
- [ ] 2.6 `CategoryColorPicker.test.tsx`: after choosing Custom, re-rendering with the palette value `#f97316` keeps Custom checked and the slider shown. Verify with `npm run test -- CategoryColorPicker`.
- [ ] 2.7 `CategoryColorPicker.test.tsx`: choosing a swatch after Custom checks the swatch and hides the slider. A swatch clears `customChosen`. Verify with `npm run test -- CategoryColorPicker`.
- [ ] 2.8 `CategoryPicker.test.tsx`: a chosen chip has a `softTint` background and `inkShade` text. Change the chosen chip's style in `CategoryPicker.tsx`. Verify with `npm run test -- CategoryPicker`.
- [ ] 2.9 Update the old-palette hexes and names in `AddCategoryDialog`, `CategoryDetail`, `VideoGrid` and `Home` tests (e.g. "Violet" → `#8b5cf6`, header tint from `#0ea5e9`). Verify with `npm run check`.

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 `it_should_refresh_a_former_solid_palette_color`: seed `#0193bf` after the chain through 0018 (new helper `migrate_to_category_color_strengthen`) → `#0ea5e9`. Write the 0019 `CASE` mapping. Verify with `cargo test sqlite_migrations`.
- [ ] 3.2 `it_should_refresh_every_former_solid_palette_color_by_position`: all 10 old hexes → the 10 new ones in order. Verify with `cargo test sqlite_migrations`.
- [ ] 3.3 Update the existing chain tests: `it_should_backfill_existing_categories_with_palette_colors_in_creation_order` gets the new hexes; `it_should_strengthen_a_former_palette_color` expects `#d3ecf8` → `#0ea5e9`; `it_should_leave_an_already_strong_color_unchanged` seeds `#03999f`; `it_should_strengthen_a_custom_pastel_keeping_its_hue` still expects `#429b5a`. Verify with `cargo test sqlite_migrations`.

## 4. Verification

- [ ] 4.1 `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, then `scripts/run-tests.sh` with no flags. Verify that all suites pass.
- [ ] 4.2 Manual (agent): first check whether a repo-root `yarrtube.sqlite3` already exists. It is the user's, so back it up or leave it untouched. Start `scripts/run-local.sh --port <free port> --fresh-videos` and create categories via `POST /api/categories`. Then use the Playwright MCP browser:
  - Open "Change color" on an Orange category, choose Custom and see the slider. Move it and check that a `PATCH` is sent with an OKLCH 0.65 / 0.2 color. Choose Teal and check that the slider hides.
  - Open an add-channel dialog, press a category chip and check its tint/ink style.
  - Take a screenshot of the picker swatches.
  - Stop the server. Remove only the DB and videos dir this run created.
- [ ] 4.3 Manual (human): judge the new palette visually in light and dark themes: sidebar dots, swatches, home chips, category header tint, chosen chips. Check that the Slate chips' bluish tint is acceptable.

## Workflow follow-up

- Archive the change (`/opsx:archive`) after review.
