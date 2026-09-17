## 1. App Shell

- [x] 1.1 In `web/index.html`, add `viewport-fit=cover` to the viewport meta tag and verify the header still renders correctly on desktop (no visual regression)
- [x] 1.2 In `web/src/App.jsx`, replace `h-screen` with `h-dvh` on the app shell root and verify, in Chrome DevTools mobile emulation with the toolbar/URL bar resize simulated, that the header stays fixed in place while scrolling any view
- [ ] 1.3 Verify on a real iOS Safari device (or BrowserStack/equivalent) that the header stays fixed while scrolling the home view, and that no double-scroll or content-ghosting artifacts appear as the address bar shows/hides mid-scroll

## 2. Detail View Video Pane On Mobile

- [x] 2.1 In `web/src/components/PlaylistDetail.jsx`, remove the `md:`-only restriction on the video pane / video list scroll-column classes so the split (video pane fixed, video list scrolls) applies at all viewport widths, and verify in mobile emulation that scrolling the video list does not move the video player or detail pane
- [x] 2.2 Apply the same change to `web/src/components/ChannelDetail.jsx` and verify the same behavior on a channel detail view
- [x] 2.3 Verify the desktop layout for both views is unchanged (video pane fixed, list scrolls, side-by-side columns at `md` and above)

## 3. Regression Check

- [x] 3.1 Run `npm run build` (or the project's configured web build command) in `web/` and verify it completes without errors
- [x] 3.2 Manually verify, in mobile emulation, the four flows shown in the original bug screenshots (home grid scroll, sidebar + tasks scroll, playlist detail video-not-downloaded state, add dialog) no longer show overlapping/ghosted content

## 4. Mobile Sidebar Drawer

- [x] 4.1 In `web/src/components/Sidebar.jsx`, accept `open`/`onClose` props and turn the sidebar into an off-canvas drawer below `md`: hidden by default (translated off-screen), positioned as a fixed overlay with its own internal scroll and a backdrop, sliding in when `open`; above `md`, keep it a static, always-visible panel unaffected by `open`
- [x] 4.2 In `web/src/App.jsx`, add sidebar open/close state and a header control (visible only below `md`) that opens the drawer, and pass the state down to `Sidebar`
- [x] 4.3 Close the drawer when a channel/playlist link is selected, when the backdrop is tapped, and via an explicit close control inside the drawer; verify the desktop sidebar (`md` and up) is completely unaffected by the new state
- [x] 4.4 In mobile emulation, verify with a long channels/playlists list that opening the sidebar scrolls only within the drawer (not the document), and that the header/video-pane fixed behavior from sections 1-2 still holds while the drawer is open and after closing it
