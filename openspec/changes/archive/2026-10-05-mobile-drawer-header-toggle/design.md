# Design

Frontend-only change (TypeScript/React/Vitest), so the sections are adapted to
`web/`: the Test Plan groups are component-behaviour tests instead of the Rust
behaviour/infrastructure split. See proposal.md for the why.

## Files

- `web/src/App.tsx` — split into `App` (just `BrowserRouter`) and exported
  `AppShell` (header + sidebar + routes + dialogs) so tests can render the
  shell under `renderWithProviders`'s `MemoryRouter`. `AppShell` owns
  `sidebarOpen`, renders `MobileNavToggle`, closes the drawer on logo click and
  on `pathname` change.
- `web/src/AppShell.test.tsx` — shell-level behaviour: toggle, logo, route
  change, header reachable while open.
- `web/src/components/MobileNavToggle.tsx` — new; header button whose three
  bars morph hamburger ↔ X (CSS transforms, `motion-reduce:transition-none`).
- `web/src/components/MobileNavToggle.test.tsx` — label / `aria-expanded` /
  click contract.
- `web/src/components/Sidebar.tsx` — remove the "Menu" title + X row; `aside`
  gets `id="app-sidebar"`; `aside` and backdrop start at
  `top-(--header-height)` instead of `inset-y-0`/`inset-0`; Escape closes while
  open.
- `web/src/components/Sidebar.test.tsx` — Escape test.

## Types & Signatures

```tsx
// components/MobileNavToggle.tsx
interface MobileNavToggleProps {
  open: boolean
  onToggle: () => void
  controls: string // id of the element it opens, for aria-controls
}
export function MobileNavToggle(props: MobileNavToggleProps): JSX.Element
// <Button variant="ghost" size="icon" className="md:hidden"
//   aria-label={open ? 'Close menu' : 'Open menu'} aria-expanded={open} aria-controls={controls}>
//   three aria-hidden <span> bars; open → top: translate-y + rotate-45,
//   middle: opacity-0, bottom: -translate-y + -rotate-45

// App.tsx
export function AppShell(): JSX.Element // everything previously inside <BrowserRouter>
export default function App(): JSX.Element // <BrowserRouter><AppShell /></BrowserRouter>

// Sidebar.tsx — props unchanged
export const SIDEBAR_ID = 'app-sidebar'
```

Stacking (mobile): header `sticky z-20` stays on top of the page; backdrop
`fixed z-30` and `aside` `fixed z-40` both start at `top-(--header-height)`,
so nothing overlaps the header. Desktop (`md:`) classes are unchanged.

## Call Stack

Toggle:
1. `MobileNavToggle.onClick` → `onToggle()` → `setSidebarOpen((o) => !o)`.
2. `Sidebar open={sidebarOpen}` translates in/out; body scroll lock unchanged.

Close on navigation:
1. Logo `<Link to="/" onClick={() => setSidebarOpen(false)}>` — covers clicking
   the logo while already on `/`.
2. `AppShell` keeps the last seen `pathname` in state and, during render, when
   it differs: `setSidebarOpen(false)` — covers any route change (Settings →
   Tasks, back button). Render-time adjustment, not `useEffect`, per the
   repo's `set-state-in-effect` lint.
3. Sidebar rows keep `onNavigate={onClose}` — covers clicking the active row
   (no route change).

Escape:
1. `Sidebar` `useEffect` on `open`: when open, `keydown` listener on `document`;
   `event.key === 'Escape'` → `onClose()`; removed on close/unmount.

## Test Plan

Group 1 — `MobileNavToggle.test.tsx` (plain `render`), one red-green cycle each:
1. Closed: button named "Open menu" with `aria-expanded="false"`.
2. Open: button named "Close menu" with `aria-expanded="true"`.
3. Clicking calls `onToggle`.

Group 2 — `Sidebar.test.tsx`:
4. Pressing Escape while open calls `onClose`.
5. The open drawer has no "Menu" title and no in-drawer "Close menu" button.

Group 3 — `AppShell.test.tsx` (`renderWithProviders`, `mockApi` for every
endpoint the shell's views touch):
6. Clicking "Open menu" opens the drawer and the control becomes "Close menu";
   clicking it again closes it ("Open menu", `aria-expanded="false"`).
7. With the drawer open, clicking the logo navigates home and closes the drawer.
8. With the drawer open on `/`, clicking the logo closes the drawer (no route change).
9. With the drawer open, navigating Settings → Tasks shows the Tasks view and closes the drawer.

Visual checks (not unit-testable in jsdom): header not covered by drawer or
backdrop, icon morph animation, reduced-motion fallback — done manually in
Verification.
