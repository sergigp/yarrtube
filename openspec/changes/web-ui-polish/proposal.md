## Why

A UI/UX review of the running app found that the web UI works but doesn't look finished yet. Specifically:

- Home cards are tinted with pale category colours that fight the thumbnails.
- The cards are framed with a box inside a box.
- Rows leave orphan cards.
- Mobile home leaves titles about 8 characters wide.
- The sidebar is noisy.
- Pages show "Loading…" text and jump when data arrives.
- The search box only filters the sidebar.

Polishing these in one pass gives the app a consistent visual language before dark mode, which is planned later.

## What Changes

- **Video cards**
  - Frameless tiles: no category tint, no border or padding.
  - Hover state: a soft tile background plus a slight thumbnail zoom.
  - **BREAKING (spec):** the "Home Card Category Tint" requirement is removed.
- **Category chips**
  - On the mixed home sections (Continue watching, Quick watches, Latest videos), each card shows its categories as small text chips.
  - Not shown in "More about <category>" sections or on a category's view.
- **Card meta line**
  - Shows the source name and the relative publish date ("Dr. Can's Chess Clinic · 2d ago").
  - Playlist cards show the playlist name with a placeholder icon, so their titles line up with channel cards.
  - The home API returns each video's publish date.
- **Stronger category palette**
  - Categories store a solid mid-strength colour.
  - The soft tint (chip and header backgrounds) and the dark text shade are derived from it.
  - **BREAKING:** the palette values change. Existing pastel colours are migrated to the stronger colour of the same hue.
  - The custom-hue option keeps the new lightness.
- **Sidebar**
  - Unwatched counts become muted numbers instead of filled green pills.
  - Row "•••" menus show on hover and focus with a mouse, and always on touch.
  - Category dots use the stronger colours.
- **Global search** (replaces the sidebar filter)
  - A search box in the header, plus a ⌘K/Ctrl+K shortcut.
  - Results are grouped: Categories, Channels, Playlists, Videos.
  - A new daemon endpoint searches video titles.
  - **BREAKING (spec):** "Sidebar Search Field" and "Sidebar Search Filtering" are removed.
- **Header**
  - The logo's size is reserved so the header doesn't jump.
  - On mobile, the header is shorter, hides while scrolling down and returns on scroll up.
  - On mobile, the announcement bar is compacted to one line.
- **Loading states**
  - Skeleton placeholders replace "Loading…" text on home, category, channel and playlist views.
  - Home shows skeleton sections instead of flashing a "Latest videos" heading.
  - The category view renders its header immediately.
- **Mobile home**
  - Sections become horizontal swipe rows, with the next card peeking in.
  - "Latest videos" becomes a one-column feed.
  - Video menu tap targets are at least 44px.
- **Desktop home**
  - Shelf sections show one row sized to the available width: no orphan cards, and "See all" for the rest.
- **Detail views**
  - Video list rows show the publish date.
  - Delete moves out of the desktop icon row into a "More actions" menu.
  - Scrollbars are thin.
- **Category view**
  - The colour circle is replaced by a "Change color" header action.
  - The header is tinted with the derived soft tint.
- **Tasks view**
  - Gets a page title and is left-aligned like the other views.
- **Typography**
  - Home section headings get a step more weight and size.

Out of scope: dark mode.

## Capabilities

### New Capabilities
- `video-search`: daemon endpoint that searches downloaded videos of tracked sources by title.

### Modified Capabilities
- `web-ui`:
  - Home layout: cards, meta line, shelves and feed, skeletons.
  - Category chips replace card tints.
  - Header: hides on mobile, reserved logo size.
  - Sidebar: count style, hover menus.
  - Global search replaces the sidebar search.
  - Detail page header moves delete into a menu.
  - Category view header and loading state.
  - Colour picker uses the stronger palette.
  - Tasks view layout.
  - Video list row dates.
- `category-crud`: new palette values, defaults, and the upgrade from the pastel palette.
- `video-listing`: home videos include their publish date.

## Impact

- **Web** (`web/src`):
  - `VideoGrid`, `Home`, `ThumbnailOverlay`, `Sidebar`, `SidebarCategories`, `SidebarSectionHeading`, `DetailHeader`, `CategoryDetail`, `CategoryColorPicker`, `VideoListPane`, `TasksView`, `App`, `AnnouncementBar`.
  - `lib/categoryColors.ts`, `api/types.ts`, `api/queries`, test helpers and builders.
  - New skeleton, chip and search components.
- **Daemon**:
  - `domain/category/category_color.rs` (palette).
  - New SQL migration that strengthens existing colours.
  - Home video DTO gains `published_at`.
  - New `GET /api/videos/search` route, use case and repository query.
- **Tests**: Vitest component tests, Rust tests for the palette, migration and search, e2e selectors that relied on "Loading…" text or the sidebar search.
- **No new dependencies.**
