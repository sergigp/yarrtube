## 1. Walking skeleton

- [x] 1.1 Create `components/MobileNavToggle.tsx` with the props from design.md rendering the current static `Menu` icon button (label "Open menu", wired to `onToggle`); split `App.tsx` into `App` + exported `AppShell` and use `MobileNavToggle` in the header with `onToggle={() => setSidebarOpen(true)}`; export `SIDEBAR_ID` from `Sidebar.tsx` and set it as the `aside` id. No behaviour change. Verify `npm run typecheck` and the existing `npm run test` suite pass. No new tests in this task.

## 2. Component behaviour (TDD)

- [x] 2.1 `MobileNavToggle` closed: named "Open menu" with `aria-expanded="false"` — verify the test fails first, then passes.
- [x] 2.2 `MobileNavToggle` open: named "Close menu" with `aria-expanded="true"`, bars morph into an X (motion-reduce safe) — verify the test fails first, then passes.
- [x] 2.3 `MobileNavToggle` click calls `onToggle` — verify the test passes.
- [x] 2.4 `Sidebar` pressing Escape while open calls `onClose` — verify the test fails first, then passes.
- [x] 2.5 `Sidebar` open drawer has no "Menu" title and no in-drawer "Close menu" button; offset `aside` and backdrop to start at `top-(--header-height)` — verify the test fails first, then passes.
- [x] 2.6 `AppShell` header control toggles the drawer open and closed (`setSidebarOpen((o) => !o)`) — verify the test fails first, then passes.
- [ ] 2.7 `AppShell` with the drawer open, clicking the logo navigates home and closes the drawer — verify the test fails first, then passes.
- [ ] 2.8 `AppShell` with the drawer open on `/`, clicking the logo closes the drawer — verify the test passes.
- [ ] 2.9 `AppShell` with the drawer open, Settings → Tasks shows the Tasks view and closes the drawer (close on `location.pathname` change) — verify the test fails first, then passes.

## 3. Pure-logic unit tests (TDD)

_None — no new pure logic._

## 4. Verification

- [ ] 4.1 Run `npm run check` (typecheck + lint + test) in `web/` and confirm it passes.
- [ ] 4.2 Manually verify in the running app via `scripts/run-local.sh` at a phone-width viewport: header stays visible and clickable over the open drawer, hamburger ↔ X animation, reduced-motion disables it, Escape/backdrop/logo/Settings → Tasks/row selection all close the drawer, desktop layout unchanged.
