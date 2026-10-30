## Context

The proposal's Why section gives the motivation. Current state that shapes the approach:

- **Home cards.** `VideoGrid` frames and tints cards when it is given `categoryColors`. `Home` always passes them; `CategoryDetail` doesn't. `lib/categoryColors.ts` holds the pastel `PALETTE`, `cardBackground` and the OKLCH helpers (`hueOf`, `pastelFromHue`).
- **Grid columns.** The grid uses viewport breakpoints (`sm/lg/xl/2xl:grid-cols-*`), so the sidebar's 224px skews the column count.
- **Palette in the daemon.** It lives in `CategoryColor::PALETTE` (`src/domain/category/category_color.rs`). Migrations are SQL files run through `rusqlite_migration` 2.6, which supports `M::up_with_hook` for Rust-side data fixes.
- **Home video data.** `GET /api/videos/home` returns `HomeVideoResponse` without `published_at`; category and source video DTOs already have it. `VideoSearcher` (`src/domain/services/video_searcher.rs`) is the read service behind every video listing.
- **Detail header.** Desktop shows icon-only buttons with Radix tooltips, delete among them. Below the desktop breakpoint there's a single "⋮" menu, already labelled. So the "labels on touch" item from the review is already met and only the desktop delete placement changes.
- **Sidebar search.** It is a local filter in `Sidebar.tsx`, shown above 15 entries.
- **Home loading.** While loading, home renders "Latest videos" with "Loading…", because that section is the only one not hidden while `videos` is undefined. That's the source of the jump.

## Goals / Non-Goals

**Goals:**
- One card component look for home and category views, with presentation switches (chips, layout) passed by the caller.
- Colour tokens derived from a single stored colour per category, so a future dark mode only re-derives them.
- Layout driven by the content area's width (container queries), not the viewport.

**Non-Goals:**
- Dark mode.
- Searching descriptions or not-yet-downloaded videos.
- A full-text index.
- Restyling dialogs and forms.
- Changing the desktop fixed video area layout of detail views (see the Scrollbars decision).
- Accent- and non-ASCII-case-insensitive video search ("falcon" finding "Falcón"). Matching is case-insensitive for ASCII only. It's a follow-up change: a normalised title column, with no API change.

## Decisions

### Cards: frameless, with explicit variants
`VideoGrid` drops `categoryColors`-driven tint and framing. It gets `chips?: Map<id, {name,color}>` and `layout: 'grid' | 'shelf' | 'feed'`. Cards render chips only when `chips` is passed, so Home passes it to the mixed sections only.

The hover treatment lives on the `<li>`:
- `group` plus `hover:bg-muted/60` with a negative-margin padding trick (`-m-2 p-2 rounded-xl`), so the hover surface extends past the content without shifting it.
- The thumbnail sits in an `overflow-hidden rounded-lg` wrapper with `group-hover:scale-[1.03] transition-transform`.

The whole card is the video's target, matching the hover surface: the title's link is stretched over the `<li>` (`after:absolute after:inset-0`, the `<li>` is `relative`), and the thumbnail is no longer a link of its own, so each card has one video link. The avatar, the playlist icon (now a link to its playlist, like a channel's avatar), the source name and the actions menu sit above it with `relative z-10`. Chips stay labels: a click on one opens the video.

*Alternative:* keep a frame on hover only. Rejected because it looks heavier, and touch devices never see it.

### Category colour model
- **Stored:** the solid colour. The new palette is the old hues at OKLCH L 0.62 / C 0.13. Taupe uses C 0.05, because sand at full chroma reads as ochre.
- **Derived in `lib/categoryColors.ts`:**
  - `softTint(hex)` = OKLCH(0.94, 0.04, hue): chip background and category header. This is today's `pastelFromHue`, renamed.
  - `inkShade(hex)` = OKLCH(0.42, 0.10, hue): chip text, at least 4.5:1 contrast on the soft tint.
  - `solidFromHue(hue)`: for the custom slider.
- All of them use the existing gamut-clamping loop.
- Swatch names change to match what they now look like: Butter→Mustard, Peach→Orange, Lilac→Violet, Aqua→Teal, Pistachio→Olive, Sand→Taupe.

*Alternative:* store both the pastel and the strong colour. Rejected: two sources of truth.

*Alternative:* store a hue only. Rejected: it breaks the existing API contract (`#rrggbb`) and arbitrary custom colours.

### Daemon migration
`0018_category_color_strengthen`:
- An SQL `UPDATE … CASE` maps the 10 old palette hexes to the new ones.
- An `M::up_with_hook` hook then reads the remaining colours and rewrites any with OKLCH L > 0.85 using a small Rust OKLCH helper next to `CategoryColor`. A port of `hueOf` and the clamping loop, unit-tested against the same values as the TS helper.

The migration test follows `it_should_backfill_existing_categories_with_palette_colors_in_creation_order`.

*Alternative:* do the custom-colour mapping client-side on first load. Rejected: it races the Plex label sync and leaves the DB inconsistent.

### Shelves and feed via container queries
- `main`'s inner wrapper becomes a Tailwind v4 `@container`.
- Shelf sections use `grid-flow-col` with container-query column counts (`@xl:grid-cols-4 @4xl:grid-cols-5 …`). Cards beyond the visible column count are hidden with `@container` `nth-child` rules, generated from one column table shared with the grid.
- Below `sm`, a shelf becomes `flex overflow-x-auto snap-x snap-mandatory` with cards at `w-[75%] shrink-0 snap-start` and a 16px scroll padding.
- Latest uses `grid-cols-1` below `sm` and the container column table above it.

*Alternative:* JS `ResizeObserver` slicing. Rejected: CSS is enough, and no re-render on resize.

*Alternative:* a carousel with arrows on desktop. Deferred; "See all" covers it.

### Relative dates
A pure `lib/relativeAge.ts` (`formatAge(published, now)` → "3h ago" / "2d ago" / "5w ago" / "4mo ago" / "2y ago"). It is unit-tested with fake timers. Used by cards and list rows.

### Skeletons
- `components/ui/skeleton.tsx`: shadcn-style `animate-pulse rounded bg-muted`, already neutralised by the global reduced-motion rule.
- Composites: `VideoCardSkeleton`, `VideoGridSkeleton({layout, count})` and `VideoRowSkeleton`.
- Loading regions get `aria-busy="true"` and a visually hidden "Loading…" `role="status"`. This keeps tests querying by role and text the way they do today, with minimal churn.
- Home renders 2 skeleton shelves plus a skeleton grid when `home` is undefined and there is no error.
- `CategoryDetail` reads the category from the `useCategories` cache to render the header immediately.

### Global search
- `components/SearchDialog.tsx` is built on the existing `ui/dialog` plus a listbox with `aria-activedescendant` keyboard navigation. No `cmdk` dependency; the list is small.
- Name groups filter the cached `useCategories`/`useChannels`/`usePlaylists` data with the same contains-ignoring-case logic the sidebar uses today. That logic moves into `lib/search.ts`.
- Videos: `useVideoSearch(q)` is a react-query query keyed by the debounced text (250 ms), enabled at 2 or more trimmed characters.
- While categories are turned off (`useCategoriesShown`, from the preferences), the Categories group is never listed.
- The ⌘K/Ctrl+K listener lives in `AppShell` and is ignored while a dialog is open (`document.querySelector('[role=dialog]')`).
- **Header:** a search button styled as a field ("Search… ⌘K") at or above `sm`, an icon button below.

*Alternative:* inline header results dropdown. Rejected: it doesn't work at phone width; a dialog does.

### Search endpoint
- `GET /api/videos/search?q=` goes to `videos::search_videos`. It validates the query (trimmed, 2 or more characters, else 400 via the existing `validation.rs`) and calls `VideoSearcher::search_by_title(text, limit 20)`.
- Repository: a new `VideoRepository::find_downloaded_by_title(text, limit)` using `title LIKE '%' || ?1 || '%' ESCAPE '\'`, with `%`, `_` and `\` escaped.
- SQLite `LIKE` is case-insensitive for ASCII only, which matches the spec; accent folding is a non-goal.
- Source resolution reuses the home dedup rule (channel first), ignoring home exclusion.
- The response reuses `HomeVideoResponse`, which gains `published_at` for home anyway, so the frontend renders results with the same type.

### Mobile header
- A `useScrollDirection` hook on `window` (mobile only, since the document scrolls there) toggles `translate-y-[-100%]` on the sticky header.
- It is disabled at the top of the page, while the sidebar is open, and while a pinned player is present. The player already offsets by `--header-height`; when the header hides, the player's `top` uses a `--header-offset` variable that becomes 0.
- The logo `img` gets explicit `width`/`height` attributes matching its intrinsic ratio.

### Sidebar
- Counts become `text-xs tabular-nums text-muted-foreground`, and `text-foreground` on the active row.
- Row menus use `opacity-0 group-hover:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100`, behind a `@media (hover: hover)` variant (`[@media(hover:hover)]:opacity-0`), so touch keeps them visible.
- The space stays reserved, so the count doesn't shift.
- The sidebar search and its threshold logic are removed.

### Scrollbars
The desktop fixed video area keeps its own scroll for the detail pane; that's the existing spec. Instead of restructuring, scrollbars get `scrollbar-width: thin` and a muted `scrollbar-color` globally, in `index.css`.

*Alternative:* one scroll area with a sticky player. Rejected: it breaks "Detail View Fixed Video Area" and its tests, and the review's complaint was visual weight.

## Risks / Trade-offs

- **The palette change is visible on Plex.** Category labels don't carry colour, so there's no impact there. External API clients that hard-code the pastel palette would see new defaults. → Release note in `CHANGELOG.md`.
- **Hover-only sidebar menus lower discoverability.** → Always shown on the active row and on touch, and keyboard focus reveals them.
- **The container-query `nth-child` hiding must stay in sync with the column table.** → Generate both from one constant and add a component test per breakpoint class.
- **Hide-on-scroll headers can be jittery on iOS rubber-banding.** → Ignore deltas under 8px and negative `scrollY`.
- **Search uses `LIKE` with no index**, scanning the videos table per keystroke burst. → A debounce, a 2-character minimum and a 20-row limit. Libraries are thousands of rows, not millions.
- **e2e tests may rely on "Loading…" text or the sidebar search field.** → Covered in the tasks; the visually hidden status text keeps most selectors working.

## Migration Plan

- The daemon migration runs once at startup and is forward-only, like the existing migrations.
- Rollback to an older image keeps the new strong colours. The old UI renders them as tints at 50% alpha: darker cards, but nothing breaks.
- No env vars change.

## Open Questions

- What exact chip and age typography (size, weight) to use? Tune in implementation against real data.
