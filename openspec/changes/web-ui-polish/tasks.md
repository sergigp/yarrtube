## 1. Daemon: palette and colour migration

- [x] 1.1 Replace `CategoryColor::PALETTE` with the strong palette; update its unit tests and the default-colour tests ("Default Category Color" scenarios) and verify `cargo test category_color` passes
- [x] 1.2 Add a Rust OKLCH helper (hex → hue/lightness, hue → solid colour with gamut clamp) beside `CategoryColor`, unit-tested with the spec's example values (Sky pastel `#d3ecf8` → `#0193bf`), and verify the tests pass
- [x] 1.3 Add migration `0018_category_color_strengthen.sql` (old palette → new palette `CASE` update) plus an `M::up_with_hook` that rewrites remaining colours with OKLCH lightness > 0.85; add migration tests for a former palette colour, a custom pastel and an already-strong colour, and update the 0013 backfill test expectations; verify `cargo test sqlite_migrations` passes

## 2. Daemon: home publish date and video search

- [x] 2.1 Add `published_at` to the home video view/DTO (`HomeVideoResponse`) for every section and verify a home endpoint test asserts it, including `null` for unknown dates
- [x] 2.2 Add `VideoRepository::find_downloaded_by_title(text, limit)` (LIKE with escaped `%`, `_`, `\`) with repository tests for matching, case, escaping, not-downloaded exclusion and limit; verify they pass
- [x] 2.3 Add `VideoSearcher::search_by_title` (one entry per YouTube video, channel source first, home exclusion ignored, newest sync first, max 20) with service tests for the "Search Videos By Title" scenarios; verify they pass
- [x] 2.4 Add route `GET /api/videos/search` with query validation (trimmed ≥ 2 chars, else 400) and handler tests for the "Search Text Validation" scenarios; verify `cargo test`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass

## 3. Web: foundations

- [x] 3.1 Update `api/types.ts` (`HomeVideo.published_at`, search response) and the fixture builders (`aHomeVideo`), add `useVideoSearch` to the queries, and verify `npm run typecheck` passes
- [x] 3.2 Rework `lib/categoryColors.ts`: new `PALETTE` (Mustard…Taupe), `softTint`, `inkShade`, `solidFromHue`; remove `cardBackground`/`CROWDED_CARD`; update `categoryColors.test.ts` and verify it passes
- [x] 3.3 Add `lib/relativeAge.ts` (`formatAge`) with fake-timer unit tests for h/d/w/mo/y boundaries and verify they pass
- [x] 3.4 Add `components/ui/skeleton.tsx` and `VideoCardSkeleton`, `VideoGridSkeleton`, `VideoRowSkeleton` (with `aria-busy` + visually hidden status text) and verify a component test finds the loading status by role
- [ ] 3.5 Add thin, muted scrollbar styles in `index.css` and reserve the header logo's size (explicit `width`/`height`); verify visually with `scripts/run-spa-against-remote.sh` (against the NAS) that the header doesn't shift on load

## 4. Web: video cards and home

- [x] 4.1 Make `VideoGrid` cards frameless with the hover surface and thumbnail zoom, remove tint and frame, and verify the updated tests assert no card background or border classes
- [x] 4.2 Add the meta line (source name · age) and the playlist placeholder icon to cards; verify tests for the channel, playlist and unknown-date scenarios
- [x] 4.3 Add category chips (max 2 + "+N", soft tint background, ink text), shown only when the caller passes chips; verify tests for the one, many, none and category-section scenarios
- [ ] 4.4 Add `layout: 'grid' | 'shelf' | 'feed'` with container-query columns, single-row shelves (no partial row), the mobile snap-scrolling shelf and the mobile one-column Latest feed; make the main content wrapper an `@container`; verify component tests for the layout classes, and check visually at 390px, 1024px and 1440px with `scripts/run-spa-against-remote.sh`
- [x] 4.5 Update `Home`: chips on Continue watching, Quick watches and Latest only; shelf layout for those and the category sections; skeleton sections while loading (no "Latest videos" flash); larger section headings; update `Home.test.tsx` for the loading, error and chip scenarios and verify it passes
- [x] 4.6 Make video menu controls ≥44px hit area on touch (`pointer: coarse`) and verify with a test on the class/variant

## 5. Web: sidebar and global search

- [x] 5.1 Restyle sidebar counts as muted tabular numerals (row and folded heading) and verify the badge tests still find counts by text
- [x] 5.2 Make row menus (channels, playlists, categories) hover/focus/active/open-visible on fine pointers and always visible on touch; update the "Sidebar Row Menu" tests and verify they pass
- [x] 5.3 Remove the sidebar search field and filtering (moving the match logic to `lib/search.ts` with unit tests); update `Sidebar.test.tsx` and verify it passes
- [x] 5.4 Build `SearchDialog` (grouped results, keyboard navigation, debounced video search, empty/error states) plus the header trigger (field at ≥sm, icon below) and the ⌘K/Ctrl+K shortcut in `AppShell`; add tests covering every "Global Search" scenario and verify they pass

## 6. Web: header, announcement, detail views, category and tasks

- [ ] 6.1 Add the mobile hide-on-scroll header (56px, `useScrollDirection`, disabled at top, with the sidebar open or a pinned player; `--header-offset` for the pinned player) and verify with a hook unit test plus a manual check at 390px with `scripts/run-spa-against-remote.sh`
- [ ] 6.2 Compact the announcement bar to one line below `sm` and verify `AnnouncementBar.test.tsx` and a visual check with `scripts/run-spa-against-remote.sh`
- [x] 6.3 Move desktop delete into a "More actions for <name>" "⋮" menu in `DetailHeader`; update `DetailHeader.test.tsx` for the desktop scenario and verify it passes
- [x] 6.4 Show the publish age on video list rows (`VideoListPane` and the category playback list) and the skeleton rows while loading; verify tests for the row date scenarios and the loading status
- [x] 6.5 `CategoryDetail`: header tinted with `softTint`, color circle replaced by a "Change color" header action, header rendered from the cached categories list while videos load (summary placeholder, skeleton grid); update `CategoryDetail.test.tsx` (including the "Violet" `#9870c5` scenario) and verify it passes
- [x] 6.6 Update `CategoryColorPicker` for the new swatch names/values and the solid custom slider; update its tests and verify they pass
- [x] 6.7 `TasksView`: add the "Tasks" heading and left-align the view; update `TasksView.test.tsx` and verify it passes

## 7. Integration

- [x] 7.1 Update e2e tests that relied on "Loading…" text, the sidebar search field or the desktop delete button; add an e2e for the header search finding a video; verify the e2e suite passes
- [x] 7.2 Add a `CHANGELOG.md` entry (new category palette, global search, UI refresh) and verify it renders
- [x] 7.3 Run `scripts/run-tests.sh` with no flags and verify every suite (web, rust, contract, e2e) passes
- [ ] 7.4 Run `scripts/run-spa-against-remote.sh` against the NAS and check home, a channel, a category, tasks and the search dialog's name groups at 390px and 1440px against the specs (card dates and video results stay empty until the NAS runs the new daemon); record any visual tweaks made
- [x] 7.5 Run `scripts/run-local.sh` and check the daemon-backed features (migrated category colours, card and row publish dates, video results in search) and verify they render as specified
