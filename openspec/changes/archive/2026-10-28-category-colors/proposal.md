# Proposal

## Why

Categories group sources, but nothing on the home page shows which category a video belongs to. Giving each category a soft color lets home cards be tinted by category, so the user can scan the home page by topic. It also prepares the ground for automatic, LLM-driven category assignment later.

## What Changes

- Every category has a color, stored as a `#rrggbb` hex. A palette of 10 soft presets (butter, peach, coral, rose, lilac, periwinkle, sky, aqua, pistachio, sand) is offered, plus a custom color.
- Creating a category accepts an optional color. When none is given, the daemon picks the least-used palette color, so categories created inline from the categories field get one too.
- Updating a category accepts a name, a color, or both (`PATCH /api/categories/:id`).
- Listed and created categories include their color.
- Existing categories get palette colors in a migration, so no category is left without a color.
- Videos on home, and in a category's video list, include the identifiers of the categories they belong to (through any member that tracks the YouTube video), in category-name order.
- Web UI:
  - The Add category dialog has a color picker (the 10 swatches plus a custom hue slider locked to pastel lightness), with the next free color preselected.
  - The category view header is tinted with the category's color and offers a control to change it.
  - Each sidebar category row shows a color dot.
  - Home cards get the category tint as their background: one category gives a flat tint, 2–3 categories give a diagonal hard-stop split in name order, 4 or more give a neutral grey, and no category leaves the card white.
  - The video grid on a category's own view stays white.

## Capabilities

### New Capabilities

### Modified Capabilities
- `category-crud`: categories gain a color (create with an optional color plus a default, update the color, list it); rename becomes update name and/or color.
- `video-listing`: home and category videos include the identifiers of their categories.
- `web-ui`: color picker in the Add category dialog, color control and tint in the category view header, color dot in the sidebar rows, category tint on home cards.

## Impact

- DB: a new migration adds `categories.color` and backfills existing rows.
- Rust: a `CategoryColor` value object and its palette; `Category`, `CategoryCreator`, `CategoryUpdater`, `CategoryView`, `HomeVideoView`, `VideoSearcher`, `SqliteCategoryRepository`; the category and video HTTP DTOs and handlers.
- API: additive fields (`color`, `category_ids`). `PATCH /api/categories/:id` no longer requires `name`, but still requires at least one field.
- Web: `api/types.ts`, the client and fixture builders, a new `lib/categoryColors.ts` (palette, card background), `AddCategoryDialog`, `CategoryDetail`, `SidebarCategories`, `VideoGrid`, and a new `CategoryColorPicker`.
