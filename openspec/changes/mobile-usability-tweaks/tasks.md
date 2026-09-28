## 1. Walking skeleton

- [x] 1.1 Convert `VideoSource` to struct variants carrying `name` (`PlaylistName` / `String`) and fill them from the loaded `Playlist` / `Channel` in `VideoSearcher`. Add `name: String` to `RecentVideoSourceResponse`, with the DTO mapping hardcoded to `String::new()`, and add `name: String::new()` to the test helpers `playlist_source` / `channel_source`. Done when `cargo build` succeeds and `cargo test --locked` passes, with no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_include_channel_name_in_recent_videos`: the recent-videos response's channel source carries the channel's name. Verified by that test plus a green `cargo test --locked`.
- [x] 2.2 `it_should_include_playlist_name_in_recent_videos`: the recent-videos response's playlist source carries the playlist's name. Verified by that test plus a green `cargo test --locked`.

## 3. Infrastructure adapters (TDD)

No adapter or repository changes in this change.

## 4. Web UI

- [ ] 4.1 Shell scrolling model: in `App.jsx`, on mobile the document scrolls and the header is `sticky top-0` with `h-[var(--header-height)]` (add `--header-height` to `index.css`). The desktop keeps the `h-dvh` shell (`md:` classes). Verify on a real iPhone with `scripts/run-local.sh`: on the home view, content scrolls under Safari's bars, the gray bars are gone and the toolbar collapses when scrolling. **Stop and rethink the approach if it doesn't.**
- [x] 4.2 Extract `VideoPlayer.jsx` and `VideoDetail.jsx` from `PlaylistDetail` / `ChannelDetail` with the current markup unchanged, and use them in both views. Verify both detail views look and behave the same as before on desktop.
- [x] 4.3 Mobile detail layout: both detail views become a single document-flow column below `md`. `VideoPlayer` is `aspect-video`, sticky at `top-[var(--header-height)]`, with no `min-h-80`, and main's top padding is reduced on mobile. The desktop grid with inner scrolling is unchanged. Verify on a phone viewport that there's no gap around the video, the player stays pinned while the details and list scroll, and the desktop layout is unchanged.
- [x] 4.4 `VideoDetail`: the title is on its own full-width row. Badges, path and "Open on YouTube" go in a collapsible section, collapsed by default below `md` and expanded at `md` and above. In a channel view the avatar links to `/channels/:id`. Verify with a long title on a phone viewport and by toggling the section.
- [x] 4.5 Sidebar: replace the hover buttons with an always-visible `...` `DropdownMenu` (Sync, Mark all watched for channels only, Delete via the existing `ConfirmDialog`). Make the drawer `w-64`, and lock the page behind it while it's open (`body` overflow). Verify on an iPhone that the menu opens on tap without navigating, the badge sits next to `...`, and the page keeps its scroll position after the drawer is dismissed.
- [x] 4.6 Home card: split it into a video link (thumbnail and title) and, for channel sources, a channel link (avatar and `source.name`). There are no nested `<a>` elements. Verify that tapping the avatar or name opens the channel and tapping the thumbnail or title opens the video.
- [ ] 4.7 App icon: replace `favicon.svg` with the Yarrtube mark. Render `apple-touch-icon.png` (180), `icon-192.png` and `icon-512.png` from it. Add `manifest.webmanifest` (`display: standalone`), plus the `theme-color`, `apple-touch-icon` and manifest links in `index.html`. Verify after `npm run build` and `cargo build` that the binary serves `/manifest.webmanifest` with `application/manifest+json`, and that iOS "Add to Home Screen" shows the icon and opens without Safari's bars.
- [x] 4.8 Smoke-test sidebar helpers (`smoke-tests/helpers/sidebar.js`): `syncItem`, `deleteItem` and `markItemWatched` open the row's `...` menu and choose the action. `syncItem` waits for the reconcile response instead of the old button becoming enabled again. Verify with `scripts/run-smoke-tests.sh`.

## 5. Verification

- [x] 5.1 `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `npm run lint` in `web/` all pass.
- [ ] 5.2 Manual pass with `scripts/run-local.sh`:
  - on a real iPhone in Safari and from the home screen: home, playlist detail, channel detail, tasks, the sidebar drawer and the Add dialog all behave as in `specs/web-ui/spec.md`
  - on desktop: the layout and sidebar are unchanged apart from the `...` menu
