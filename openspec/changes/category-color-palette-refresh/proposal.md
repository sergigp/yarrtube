# Proposal

## Why

The color picker's "Custom" option can't always be chosen: on a category colored Orange or Periwinkle, clicking Custom does nothing, and while in Custom, sliding onto a palette hue (e.g. 61 → Orange) flips back to that swatch and hides the slider. Separately, the palette was tuned for card backgrounds: all 10 colors share one lightness and chroma (OKLCH 0.62 / 0.13), so as dots, swatches and chips they look alike.

## What Changes

- Fix: "Custom" stays chosen once the user picks it, whatever hex the slider lands on, until a swatch is chosen.
- New palette of 10 distinct, standard, vivid colors (Tailwind-500-like): Amber `#f59e0b`, Orange `#f97316`, Red `#ef4444`, Pink `#ec4899`, Violet `#8b5cf6`, Blue `#3b82f6`, Sky `#0ea5e9`, Teal `#14b8a6`, Green `#22c55e`, Slate `#64748b`. Order matches the old palette's hue families, so defaults and migration stay positional.
- Custom colors use a fixed vivid OKLCH lightness 0.65 / chroma 0.2 (chroma lowered to fit sRGB), instead of the old palette's 0.62 / 0.13.
- On upgrade, categories colored with an old palette color get the new palette color at the same position. Other colors are left unchanged.
- Chosen chips in the Categories field use the soft tint + dark shade of the category's hue (like home card chips) instead of the solid fill, so their text stays readable on vivid colors.

## Capabilities

### New Capabilities

### Modified Capabilities
- `category-crud`: palette values, default-color scenarios, new upgrade requirement remapping old palette colors.
- `web-ui`: color picker names and custom lightness/chroma, Custom stays selected; add-category and category-view scenarios use new names/hexes; chosen category chips use tint + shade.

## Impact

- Rust: `src/domain/category/category_color.rs` (`PALETTE`), new `migrations/0019_category_palette_refresh.sql`, `src/infrastructure/shared/sqlite_migrations.rs` (register + tests), `src/application/http/categories/mod.rs` tests.
- Web: `web/src/lib/categoryColors.ts`, `web/src/components/CategoryColorPicker.tsx`, `web/src/components/CategoryPicker.tsx`, fixture `web/src/test/helpers.tsx` and affected tests.
- No API shape change; stored colors of existing categories using the old palette change once on upgrade.
