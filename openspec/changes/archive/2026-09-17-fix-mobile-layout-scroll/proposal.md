## Why

Since the SPA redesign (#30), the mobile layout relies on `h-screen` (`100vh`) to size the app shell so only the `<main>` region scrolls, keeping the header and the detail-view video pane in place. On iOS Safari, `100vh` reflects the largest possible viewport (toolbar collapsed), not the currently visible one, so the app shell ends up taller than the visible area whenever the toolbar is showing. The page itself then becomes scrollable in addition to `<main>`, so the header scrolls away instead of staying fixed, and the two overlapping scroll contexts repaint out of sync, producing visible content-ghosting glitches while scrolling (confirmed against user-supplied screenshots). Separately, the playlist/channel detail view's fixed-video-pane behavior is implemented with `md:`-prefixed classes only, so on mobile the video pane and video list scroll together as one block instead of the pane staying fixed — the same defect pattern, and already inconsistent with the existing "Detail View Fixed Video Area" requirement, which does not carve out mobile.

## What Changes

- Replace `h-screen` with `h-dvh` (dynamic viewport height) on the app shell so its total height tracks the real visible viewport as Safari's toolbar shows/hides, restoring single-scroll-container containment on mobile the way it already works on desktop.
- Add `viewport-fit=cover` to the viewport meta tag so the fixed header doesn't sit under a notch/Dynamic Island on mobile.
- Extend the playlist/channel detail view's independent-scroll-column layout (video pane fixed, video list scrolls) to mobile viewports, removing the current `md:`-only restriction.
- Turn the sidebar into a collapsible off-canvas drawer on mobile viewports (hidden by default, opened via a header control, dismissible via a close control/backdrop/navigation), instead of the current always-stacked-above-content layout. On mobile the sidebar has no height cap, so a long channels/playlists list grows past the available space and reintroduces document-level scroll — breaking the persistent-header requirement below via a second path. Desktop (`md` and up) is unaffected: the sidebar remains a persistent, always-visible side panel.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `web-ui`: the existing "Detail View Fixed Video Area" requirement currently has no viewport carve-out but is only implemented above the `md` breakpoint; this change makes the requirement hold on mobile viewports too. Also adds a new requirement that the header stays fixed in place (visible, non-scrolling) across all viewport sizes, formalizing existing desktop behavior and fixing it on mobile. Also adds a new requirement that the sidebar is a collapsible, hidden-by-default drawer on mobile viewports that never forces document-level scrolling, while remaining a persistent, always-visible panel on desktop.

## Impact

- `web/index.html`: viewport meta tag.
- `web/src/App.jsx`: app shell height unit (`h-screen` → `h-dvh`); adds mobile sidebar-toggle state and a header control.
- `web/src/components/PlaylistDetail.jsx`, `web/src/components/ChannelDetail.jsx`: mobile scroll-column classes for the video pane / video list split.
- `web/src/components/Sidebar.jsx`: mobile off-canvas drawer behavior (open/close state, backdrop, close-on-navigate), desktop layout unchanged.
- No backend, API, or data changes.
