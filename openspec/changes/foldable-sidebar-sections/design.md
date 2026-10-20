## Files

- `web/src/hooks/useRememberedFlag.ts` (+ `.test.ts`) — per-browser boolean in `localStorage`; replaces `Sidebar.tsx`'s private `useExpandedState`.
- `web/src/lib/sidebarSections.ts` (+ `.test.ts`) — `sectionView` moves here from `Sidebar.tsx` (fold-aware, collapse removed) and `unwatchedTotal` is added; `collapse`, `channelLeadCount`, `UNREAD_CAP`, `CAUGHT_UP_PREVIEW` and their tests are deleted.
- `web/src/components/Sidebar.tsx` — heading becomes the fold toggle (chevron, muted total, active marker); folded section renders the heading only; "Show N more"/"Show less" control removed.
- `web/src/components/Sidebar.test.tsx` — fold behaviour; the "show more" test is replaced.

- `smoke-tests/helpers/sidebar.js`, `smoke-tests/tests/{addDialogLocation,playlist,channel}.spec.js` — section selectors become `h3:has(button:text-is("<Section>"))`: Playwright's `:text-is` matches the smallest element holding the text, which is now the heading button.
- `smoke-tests/tests/sidebarFolding.spec.js` (new) — folding survives a real reload, on desktop and in the mobile drawer; `smoke-tests/tests/channel.spec.js` — the folded Channels heading is marked active on the channel's route and its total drops when the video is marked watched; `smoke-tests/helpers/sidebar.js` gains `sectionHeading`; `smoke-tests/README.md` lists the new coverage.

No backend, API type or fixture changes.

## Types & Signatures

```ts
// web/src/hooks/useRememberedFlag.ts
/** Storage unreadable or unwritable: starts at `initial`, lasts until reload. */
export function useRememberedFlag(key: string, initial: boolean): readonly [boolean, () => void]

// key: `yarrtube.sidebar.folded.<section>`, initial false
```

```ts
// web/src/lib/sidebarSections.ts
export const SEARCH_THRESHOLD = 15 // unchanged

export interface SectionViewOptions {
  searchText: string
  folded: boolean
  activeId: string | null
}

export interface SectionView {
  /** True when only the heading shows: folded and not searching. */
  folded: boolean
  /** Folded, and the viewed entry belongs to this section. */
  headingActive: boolean
  /** Every row, the search matches, or `[]` when folded; undefined while loading. */
  rows: LibraryItem[] | undefined
}

export function sectionView(items: LibraryItem[] | undefined, options: SectionViewOptions): SectionView

/** Sum of the rows' unwatched counts (missing counts as 0). */
export function unwatchedTotal(items: LibraryItem[]): number

// unchanged: orderChannels, matchesSearch
```

```ts
// web/src/components/Sidebar.tsx — SidebarSectionProps
// removed: hiddenCount, expanded, collapsible, onToggleExpanded
folded: boolean
headingActive: boolean
onToggleFolded: () => void
/** Channels only; omitted for Playlists. Shown when folded and > 0. */
foldedTotal?: number | undefined
```

Heading markup: `<h3><button aria-expanded={!folded} aria-current={headingActive || undefined}>` containing `ChevronRight`/`ChevronDown`, the title, and the muted total with `aria-label="<n> unwatched"`.

## Call Stack

Fold toggle:
1. `SidebarSection` heading button `onClick` → `onToggleFolded()`
2. `useRememberedFlag('yarrtube.sidebar.folded.channels', false)` toggle → `setState(next)`, `localStorage.setItem(key, String(next))` (errors swallowed)
3. `Sidebar` re-renders → `sectionView(orderedChannels, { searchText, folded, activeId })`
4. `SidebarSection` with `view.folded` → renders the heading only, `foldedTotal = unwatchedTotal(channels)`

Active entry while folded:
1. route `/channels/:id` → `activeIdFrom(pathname, '/channels/')`
2. `sectionView(...)` → `headingActive = folded && !searchText && items.some(i => i.id === activeId)`
3. heading renders with `aria-current="true"` and active styling

Search while folded:
1. `searchText` non-empty → `sectionView` returns `folded: false` with the matching rows; the remembered flag is untouched
2. clearing the search → `folded` comes from the flag again

## Test Plan

1. `sidebarSections.test.ts`
   - `sectionView` › lists every row when unfolded
   - `sectionView` › shows nothing but the heading when folded
   - `sectionView` › shows every match when folded but searching
   - `sectionView` › marks the heading active when folded and the active row belongs to the section
   - `sectionView` › does not mark the heading active when unfolded
   - `sectionView` › does not mark the heading active when the active id is not in the section
   - `unwatchedTotal` › sums the rows' unwatched counts, treating missing counts as 0
2. `useRememberedFlag.test.ts`
   - starts at the initial value when nothing is remembered
   - restores the remembered value
   - remembers a toggle under its key
   - falls back to the initial value when storage throws, and still toggles
3. `Sidebar.test.tsx`
   - lists every channel of an unfolded section with no show-more control
   - folds a section from its heading, hiding its rows and add control
   - unfolds a folded section
   - folds sections independently
   - exposes the fold state on the heading control (`aria-expanded`)
   - remembers a folded section across remounts
   - shows the channels' muted unwatched total only while folded and above 0
   - shows no total on a folded Playlists heading
   - keeps a folded section folded and marks its heading active on that section's detail route
   - shows matches of a folded section while searching, and folds it again when cleared
