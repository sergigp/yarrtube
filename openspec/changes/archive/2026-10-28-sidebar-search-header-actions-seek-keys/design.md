# Design

## Context

Web UI only; see proposal.md. Behaviour already lives on the `frontend-tweaks` branch.

## Goals / Non-Goals

**Goals:** one search over all three sidebar sections; one action list rendered two ways (desktop buttons / mobile menu); page-wide arrow-key seeking with feedback.

**Non-Goals:** sidebar row menus (still "⋮" everywhere); other player shortcuts (space, j/k/l, volume); backend changes.

## Decisions

- Header actions render from one `HeaderAction[]`, picked by `useIsDesktop()` instead of CSS-hiding both, so each action exists once in the accessibility tree.
- Seek keys listen on `document` (not the `<video>`), so they work without focusing the player, and `preventDefault()` replaces the browser's own 5 s step.
- The player container switches `md:static` → `md:relative md:top-auto md:z-auto` to anchor the overlay; `top-auto`/`z-auto` cancel the mobile sticky offset and stacking on desktop (otherwise the player shifts down over the details below).

## Files

- `web/src/lib/sidebarSections.ts` — `sectionView` made generic so categories reuse it.
- `web/src/components/Sidebar.tsx` — search above Categories; categories in threshold and "nothing matches".
- `web/src/components/SidebarCategories.tsx` — takes `searchText`, filters/unfolds, hides with no match.
- `web/src/components/DetailHeader.tsx` — builds the action list; desktop icon buttons + tooltips, mobile "⋮".
- `web/src/components/CategoryDetail.tsx` — "Manage sources" as an `extraActions` entry.
- `web/src/components/EntryActionsMenu.tsx` — doc only: now sidebar rows only.
- `web/src/lib/seekKeys.ts` — pure key → direction and clamped seek time.
- `web/src/hooks/useSeekKeys.ts` — document keydown → seek + feedback state.
- `web/src/components/VideoPlayer.tsx` — wires the hook, renders `SeekIndicator`.
- `web/src/index.css` — `seek-feedback` / `seek-chevron` keyframes.
- `web/src/test/viewport.ts`, `web/src/test/setup.ts` — `matchMedia` stub switchable to desktop per test.
- `e2e-tests/tests/{search,seekKeys,detailHeader,mobileLayout,…}.spec.js` — coverage; header specs click buttons instead of the "⋮".

## Types & Signatures

```ts
// lib/sidebarSections.ts
export interface SectionView<T> { folded: boolean; headingActive: boolean; rows: T[] | undefined }
export function sectionView<T extends { id: string; name: string }>(items: T[] | undefined, options: SectionViewOptions): SectionView<T>

// components/SidebarCategories.tsx
interface SidebarCategoriesProps { onAddCategory: () => void; onClose: () => void; searchText: string }

// components/DetailHeader.tsx
export interface HeaderAction { label: string; icon: ReactNode; onSelect: () => void; disabled?: boolean }
// DetailHeaderProps: `menuItems?: ReactNode` → `extraActions?: HeaderAction[]`

// lib/seekKeys.ts
export const SEEK_SECONDS = 10
export const SEEK_FEEDBACK_MS = 700
export type SeekDirection = 'back' | 'forward'
export function seekDirection(event: KeyboardEvent): SeekDirection | null
export function seekTime(currentTime: number, duration: number, direction: SeekDirection): number

// hooks/useSeekKeys.ts
export interface SeekFeedback { direction: SeekDirection; seconds: number; press: number }
export function useSeekKeys(videoElement: HTMLVideoElement | null): SeekFeedback | null

// test/viewport.ts
export const viewport: { desktop: boolean }
export function useDesktopViewport(): void
```

## Call Stack

- Search: `Sidebar` (`search` → trimmed `searchText` when above threshold) → `SidebarCategories({ searchText })` → `sectionView(categories, { searchText, folded, activeId })`; channels and playlists likewise.
- Header: `ChannelDetail | PlaylistDetail | CategoryDetail` → `DetailHeader(props)` → actions `[sync, …extraActions, markWatched, home, unwatchedCount, edit]` → `useIsDesktop()` ? `IconAction` per action + Delete : `DropdownMenu` items + Delete.
- Seek: `document keydown` → `seekDirection(event)` → `element.currentTime = seekTime(currentTime, duration, direction)` → `setFeedback({ direction, seconds, press })` → `VideoPlayer` renders `<SeekIndicator key={press}>` → timer clears after `SEEK_FEEDBACK_MS`.

## Test Plan

1. Behaviour (Vitest, `web/src`)
   - `sidebarCategories: it_should_filter_categories_and_hide_section_without_match`
   - `sidebarCategories: it_should_show_matches_of_a_folded_section_while_searching`
   - `sidebar: it_should_put_search_above_categories_and_count_them_toward_threshold`
   - `detailHeader: it_should_keep_every_action_including_sync_behind_the_menu_on_mobile`
   - `detailHeader: it_should_lay_actions_out_as_icon_buttons_without_menu_on_desktop`
   - `detailHeader: it_should_name_an_icon_button_in_a_tooltip_on_hover`
   - `detailHeader: it_should_run_sync_and_alert_on_failure_on_desktop`
   - `playlistDetail: it_should_offer_every_action_in_the_menu_on_mobile / as_buttons_on_desktop`
   - `seekKeys: it_should_read_arrows_as_directions_and_ignore_modified_fields_and_handled_keys`
   - `seekKeys: it_should_seek_ten_seconds_within_the_video`
   - `videoPlayer: it_should_seek_ten_seconds_on_arrow_keys`
   - `videoPlayer: it_should_add_up_quick_presses_and_fade_out`
   - `videoPlayer: it_should_leave_arrows_typed_into_a_field_alone`
2. End-to-end (Playwright)
   - `search.spec: sidebar search sits above the categories and filters them too`
   - `detailHeader.spec: every header action is a button on desktop, with no ⋮`
   - `seekKeys.spec: ← / → seek 10 s with on-screen feedback`
   - `mobileLayout.spec: header Sync through the ⋮ @mobile`
