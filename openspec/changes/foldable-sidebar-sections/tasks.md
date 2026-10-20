## 1. Walking skeleton

- [x] 1.1 Create `web/src/hooks/useRememberedFlag.ts` (returns `[initial, () => {}]`), add `unwatchedTotal` (returns 0) to `web/src/lib/sidebarSections.ts`, and move `sectionView` there from `Sidebar.tsx` with its current logic, accepting `folded` and returning `folded: false` and `headingActive: false`. Add the new `SidebarSectionProps` (`folded`, `headingActive`, `onToggleFolded`, `foldedTotal`) and wire them from `Sidebar` through `useRememberedFlag('yarrtube.sidebar.folded.<section>', false)`; the heading stays a plain `<h3>`. Collapse code stays until 2.1. Done when `npm run check` passes with no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `sectionView` › lists every row when unfolded — drives removing the row-level collapse: delete `collapse`, `channelLeadCount`, `UNREAD_CAP`, `CAUGHT_UP_PREVIEW`, `useExpandedState`, the `expanded`/`leadCount`/`hiddenCount`/`collapsible` fields and their tests
- [x] 2.2 `sectionView` › shows nothing but the heading when folded
- [x] 2.3 `sectionView` › shows every match when folded but searching
- [x] 2.4 `sectionView` › marks the heading active when folded and the active row belongs to the section
- [x] 2.5 `sectionView` › does not mark the heading active when unfolded
- [x] 2.6 `sectionView` › does not mark the heading active when the active id is not in the section
- [x] 2.7 `unwatchedTotal` › sums the rows' unwatched counts, treating missing counts as 0
- [x] 2.8 `Sidebar` › lists every channel of an unfolded section with no show-more control — replaces the "show more" test and removes the "Show N more"/"Show less" control
- [x] 2.9 `Sidebar` › folds a section from its heading, hiding its rows and add control — heading becomes the toggle button with a chevron
- [x] 2.10 `Sidebar` › unfolds a folded section
- [x] 2.11 `Sidebar` › folds sections independently
- [x] 2.12 `Sidebar` › exposes the fold state on the heading control (`aria-expanded`)
- [x] 2.13 `Sidebar` › remembers a folded section across remounts
- [x] 2.14 `Sidebar` › shows the channels' muted unwatched total only while folded and above 0
- [x] 2.15 `Sidebar` › shows no total on a folded Playlists heading
- [x] 2.16 `Sidebar` › keeps a folded section folded and marks its heading active on that section's detail route
- [x] 2.17 `Sidebar` › shows matches of a folded section while searching, and folds it again when cleared

## 3. Infrastructure adapters (TDD)

`useRememberedFlag` (browser `localStorage`):

- [x] 3.1 `useRememberedFlag` › starts at the initial value when nothing is remembered
- [x] 3.2 `useRememberedFlag` › restores the remembered value
- [x] 3.3 `useRememberedFlag` › remembers a toggle under its key
- [x] 3.4 `useRememberedFlag` › falls back to the initial value when storage throws, and still toggles

## 4. Verification

- [x] 4.1 `npm run check` (typecheck, lint, tests) and `npm run build` pass in `web/`; `cargo build --release` embeds the new build
- [x] 4.2 `scripts/run-smoke-tests.sh` passes (mobile sidebar included)
- [x] 4.4 Smoke `tests/sidebarFolding.spec.js`: a folded section stays folded across a reload while the other keeps its state (desktop), and the same in the mobile drawer
- [x] 4.5 Smoke `tests/channel.spec.js` "mark its video watched": the folded Channels heading is active on the channel route and shows a total of 1, which disappears once the video is marked watched
- [x] 4.3 Manual check with `scripts/run-local.sh`: fold/unfold both sections on desktop and in the mobile drawer, folded Channels total updates after watching a video, folded heading marked active on a channel route, search reveals matches in a folded section, fold state survives a reload
