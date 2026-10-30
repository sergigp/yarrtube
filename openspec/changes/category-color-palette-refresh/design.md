# Design

## Files

- `src/domain/category/category_color.rs`: change `PALETTE` to the new hexes. `strengthened()` and its 0.62 / 0.13 constants stay as they are: only the 0018 hook uses them, and 0019 then maps their result.
- `migrations/0019_category_palette_refresh.sql`: new. Positional `UPDATE … CASE` from old solid palette to new, like 0018. SQL only, no hook.
- `src/infrastructure/shared/sqlite_migrations.rs`: register 0019 after 0018. Update expected hexes in the existing color tests (they run the whole chain). Add 0019 tests. Add `migrate_to_category_color_strengthen` helper (the chain through 0018) to seed old solid colors.
- `src/application/http/categories/mod.rs`: tests only. Default-color expectations move to new hexes.
- `web/src/lib/categoryColors.ts`: new `PALETTE` (names + hexes). `solidFromHue` uses `CUSTOM = { lightness: 0.65, chroma: 0.2 }` (renamed from `SOLID`). `softTint` / `inkShade` unchanged.
- `web/src/components/CategoryColorPicker.tsx`: `customChosen` state. The picker shows Custom when `customChosen || !inPalette(value)`. A swatch clears `customChosen`; Custom and the slider set it.
- `web/src/components/CategoryPicker.tsx`: chosen chip uses `softTint` background + `inkShade` text instead of the solid fill.
- `web/src/test/helpers.tsx`: fixture category color `#9f8401` → `#f59e0b`.
- Web tests: `categoryColors.test.ts`, `CategoryColorPicker.test.tsx`, `CategoryPicker.test.tsx`, `AddCategoryDialog.test.tsx`, `CategoryDetail.test.tsx`, `VideoGrid.test.tsx`, `Home.test.tsx`. Update the hexes and names that came from the old palette.

## Types & Signatures

```rust
// src/domain/category/category_color.rs
impl CategoryColor {
    pub const PALETTE: [&'static str; 10] = [
        "#f59e0b", "#f97316", "#ef4444", "#ec4899", "#8b5cf6",
        "#3b82f6", "#0ea5e9", "#14b8a6", "#22c55e", "#64748b",
    ];
}

// src/infrastructure/shared/sqlite_migrations.rs
const CATEGORY_PALETTE_REFRESH_SQL: &str =
    include_str!("../../../migrations/0019_category_palette_refresh.sql");
```

```ts
// web/src/lib/categoryColors.ts
export const PALETTE: readonly { name: string; hex: string }[] // Amber, Orange, Red, Pink, Violet, Blue, Sky, Teal, Green, Slate
export function solidFromHue(hue: number): string           // OKLCH 0.65 / 0.2, chroma lowered to fit sRGB

// web/src/components/CategoryColorPicker.tsx (internal)
const [customChosen, setCustomChosen] = useState(() => !inPalette(value))
const custom = customChosen || !inPalette(value)
```

## Call Stack

- Upgrade: `SqliteDatabase` open → `sqlite_migrations::apply(conn)` → … → 0018 (pastel → old solid, light customs → 0.62/0.13) → 0019 `UPDATE categories SET color = CASE color WHEN '<old i>' THEN '<new i>' … ELSE color END`.
- Default color: `POST /api/categories` without color → `CategoryCreator::create` → `CategoryColor::least_used(used)` over the new `PALETTE`.
- Picker Custom: click Custom radio → `onSelect` → `setCustomChosen(true)`; `chooseHue(hue)` → `onChange(solidFromHue(hue))`. Slider `onChange(event)` → `chooseHue(Number(value))`. Swatch `onSelect` → `setCustomChosen(false)`; `onChange(color.hex)`.
- Chosen chip: `CategoryPicker` → `style={{ background: softTint(category.color), color: inkShade(category.color) }}`.

## Test Plan

1. Behaviour tests
   - Rust `src/application/http/categories/mod.rs`
     - `it_should_create_the_first_category_with_the_first_palette_color` (existing, updated): stored and returned color `#f59e0b`.
     - `it_should_create_a_category_with_the_least_used_palette_color` (existing, updated): `["#f59e0b", "#ef4444"]` and every color + amber again → `#f97316` both times.
   - Web `CategoryColorPicker.test.tsx`
     - offers swatches Amber … Slate, then Custom.
     - choosing Sky reports `#0ea5e9` and checks it.
     - choosing Custom from Orange shows Custom checked and the slider.
     - choosing Custom from Blue shows Custom checked and the slider.
     - after choosing Custom, a palette hex coming back as `value` (re-render with `#f97316`) keeps Custom checked and the slider shown.
     - choosing a swatch after Custom checks the swatch and hides the slider.
     - existing non-palette color opens on Custom at its hue (existing, updated hexes).
   - Web `categoryColors.test.ts`: palette names/hexes; default color expectations; `solidFromHue` round-trips hue within 3°; soft tint ≥ 200 per channel and ink ≥ 4.5:1 for every new palette color (existing, updated).
   - Web `CategoryPicker.test.tsx`: chosen chip has `softTint` background and `inkShade` text (updated).
   - Web: `AddCategoryDialog`, `CategoryDetail`, `VideoGrid`, `Home` tests: old-palette hexes and names replaced.
2. Infrastructure tests: `src/infrastructure/shared/sqlite_migrations.rs`
   - `it_should_backfill_existing_categories_with_palette_colors_in_creation_order` (existing, updated): new hexes, wrapping to `#f59e0b`.
   - `it_should_strengthen_a_former_palette_color` (existing, updated): `#d3ecf8` → `#0ea5e9`.
   - `it_should_strengthen_a_custom_pastel_keeping_its_hue` (existing): unchanged `#429b5a`.
   - `it_should_leave_an_already_strong_color_unchanged` (existing, updated): seed `#03999f`, stays `#03999f`.
   - `it_should_refresh_a_former_solid_palette_color`: seeded after 0018 with `#0193bf` → `#0ea5e9`.
   - `it_should_refresh_every_former_solid_palette_color_by_position`: all 10 old hexes → the 10 new ones in order.
