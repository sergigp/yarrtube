## Why

The sidebar is about to gain a third section (Categories) above Channels and
Playlists, and a long library already pushes Playlists far down. Letting the
user fold a whole section away gives the space back on their terms, and a
glanceable unwatched total on the folded heading means nothing is lost by
folding. Folding also makes the existing row-level collapse redundant: one
user-driven way to hide rows replaces an automatic one with its special
cases.

## What Changes

- Each sidebar section heading (Channels and Playlists now, Categories once
  it exists) becomes a toggle: a chevron (`>` folded, `v` unfolded) plus the
  section label, the whole heading row being the click target. Controls that
  live in a heading (e.g. a future manage gear) stay separate and do not
  toggle. The heading exposes its folded/unfolded state to assistive tech.
- Folding a section hides everything under its heading: its rows and its
  "+ Add ..." entry. **BREAKING** (spec): the add entries are no longer
  "always visible", only while their section is unfolded.
- **BREAKING** (spec): the row-level collapse is removed. An unfolded
  section lists all its rows; there is no leading-rows preview, no
  "Show N more"/"Show less", no remembered expanded state, and no special
  rule keeping the active entry visible in a collapsed section.
- Fold state is remembered per section, per browser, and survives reloads.
  Every section starts unfolded.
- A folded Channels heading shows, in a muted colour, the sum of its
  channels' unwatched counts; nothing is shown when the sum is 0 or while
  the section is unfolded. The Playlists total arrives with playlist
  unwatched counts (separate change).
- An active entry inside a folded section does not unfold it; the folded
  heading is marked active instead.
- Search keeps working across folded sections: matches show while
  searching, without changing the remembered fold state.
- Same behaviour in the mobile slide-in sidebar.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `web-ui`: sidebar sections gain foldable headings with a folded unwatched
  total and an active marker; "Sidebar Add Entries" and "Sidebar Search"
  account for folded sections; "Collapsed Sidebar Sections", "Remembered
  Sidebar Section State" and "Active Sidebar Entry Always Visible" are
  removed.

## Impact

- Web UI only: `Sidebar.tsx` (+ tests), `src/lib/sidebarSections.ts`
  (collapse helpers removed, fold-aware section view and total added), a
  per-viewer `localStorage` entry for fold state. No smoke test relies on
  the removed collapse; smoke selectors that found a section by its `<h3>`
  text now match the heading's toggle button instead.
- No backend change: the Channels total is derived from the existing
  per-channel unwatched counts. Old `yarrtube.sidebar.expanded.*` storage
  entries are left in place and ignored.
