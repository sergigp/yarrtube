## 1. Walking skeleton

- [x] 1.1 Create `web/src/hooks/useRememberedFlag.ts` (returns `[initial, () => {}]`), add `unwatchedTotal` (returns 0) to `web/src/lib/sidebarSections.ts`, and move `sectionView` there from `Sidebar.tsx` with its current logic, accepting `folded` and returning `folded: false` and `headingActive: false`. Add the new `SidebarSectionProps` (`folded`, `headingActive`, `onToggleFolded`, `foldedTotal`) and wire them from `Sidebar` through `useRememberedFlag('yarrtube.sidebar.folded.<section>', false)`; the heading stays a plain `<h3>`. Collapse code stays until 2.1. Done when `npm run check` passes with no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `sectionView` › lists every row when unfolded — drives removing the row-level collapse: delete `collapse`, `channelLeadCount`, `UNREAD_CAP`, `CAUGHT_UP_PREVIEW`, `useExpandedState`, the `expanded`/`leadCount`/`hiddenCount`/`collapsible` fields and their tests
- [x] 2.2 `sectionView` › shows nothing but the heading when folded
- [x] 2.3 `sectionView` › shows every match when folded but searching
- [x] 2.4 `sectionView` › marks the heading active when folded and the active row belongs to the section
- [ ] 2.5 `sectionView` › does not mark the heading active when unfolded
- [ ] 2.6 `sectionView` › does not mark the heading active when the active id is not in the section
- [ ] 2.7 `unwatchedTotal` › sums the rows' unwatched counts, treating missing counts as 0
- [ ] 2.8 `Sidebar` › lists every channel of an unfolded section with no show-more control — replaces the "show more" test and removes the "Show N more"/"Show less" control
- [ ] 2.9 `Sidebar` › folds a section from its heading, hiding its rows and add control — heading becomes the toggle button with a chevron
- [ ] 2.10 `Sidebar` › unfolds a folded section
- [ ] 2.11 `Sidebar` › folds sections independently
- [ ] 2.12 `Sidebar` › exposes the fold state on the heading control (`aria-expanded`)
- [ ] 2.13 `Sidebar` › remembers a folded section across remounts
- [ ] 2.14 `Sidebar` › shows the channels' muted unwatched total only while folded and above 0
- [ ] 2.15 `Sidebar` › shows no total on a folded Playlists heading
- [ ] 2.16 `Sidebar` › keeps a folded section folded and marks its heading active on that section's detail route
- [ ] 2.17 `Sidebar` › shows matches of a folded section while searching, and folds it again when cleared

## 3. Infrastructure adapters (TDD)

`useRememberedFlag` (browser `localStorage`):

- [ ] 3.1 `useRememberedFlag` › starts at the initial value when nothing is remembered
- [ ] 3.2 `useRememberedFlag` › restores the remembered value
- [ ] 3.3 `useRememberedFlag` › remembers a toggle under its key
- [ ] 3.4 `useRememberedFlag` › falls back to the initial value when storage throws, and still toggles

## 4. Verification

- [ ] 4.1 `npm run check` (typecheck, lint, tests) and `npm run build` pass in `web/`; `cargo build --release` embeds the new build
- [ ] 4.2 `scripts/run-smoke-tests.sh` passes (mobile sidebar included)
- [ ] 4.3 Manual check with `scripts/run-local.sh`: fold/unfold both sections on desktop and in the mobile drawer, folded Channels total updates after watching a video, folded heading marked active on a channel route, search reveals matches in a folded section, fold state survives a reload
