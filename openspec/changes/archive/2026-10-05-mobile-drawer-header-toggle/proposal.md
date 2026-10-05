## Why

On mobile, the open sidebar drawer and its backdrop cover the header, so the
logo (home) and settings are unreachable until the drawer is dismissed, and the
drawer spends its top row on a redundant "Menu" title and close button.

## What Changes

- The mobile drawer and its backdrop start below the header; the header stays
  visible and usable while the drawer is open.
- Remove the drawer's "Menu" title and in-drawer close (X) button.
- The header's menu control becomes an open/close toggle: it reflects the
  drawer state (`aria-expanded`, label "Open menu" / "Close menu") and its
  hamburger icon animates into an X while open (no animation under
  reduced-motion).
- The drawer closes on: the header toggle, the backdrop, Escape, selecting a
  sidebar entry (unchanged), the logo, and any route change (e.g. Settings →
  Tasks).

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `web-ui`: "Collapsible Mobile Sidebar" — header stays usable over the open
  drawer, the header control toggles and reflects the drawer state, and the
  drawer also dismisses on Escape, the logo, and any navigation.

## Impact

- Frontend only (`web/`): `App.tsx` (header toggle, close-on-navigation,
  extracted routed shell for testing), `components/Sidebar.tsx` (drop drawer
  header, offset below header, Escape), new `components/MobileNavToggle.tsx`.
- No API, backend, or dependency changes.
