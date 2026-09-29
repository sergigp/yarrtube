## 1. Walking skeleton

- [x] 1.1 Create every file and signature from design.md's Files and Types & Signatures, wired end to end with today's behaviour kept:
  - Move `VideoQualityField` to its own file.
  - Split `AddDialog` into `AddPlaylistDialog` (the playlist form, no tabs) and `AddChannelDialog` (today's channel form, no tabs). Delete `AddDialog.jsx`.
  - Add `SettingsMenu` with the `Tasks` item.
  - In `App`, add the `addDialog` state and replace the header `Tasks`/`Add` buttons with `SettingsMenu`.
  - Give `Sidebar`/`SidebarSection` the "Add channel"/"Add playlist" buttons (`onClose()`, then `onAdd()`).
  - Add the `showDestination` prop to `LocationField`, still unused.
  - Add a `channelNoticeLead` that returns a fixed string.
  - Switch the smoke helpers to `openAddChannelDialog`/`openAddPlaylistDialog` (sidebar buttons, no tab clicks) and `tasks.spec.js` to the `Settings` menu.

  Done when `npm run build` and `npm run lint` pass in `web/`, and `scripts/run-smoke-tests.sh` passes with the existing specs.

## 2. Behaviour (TDD)

Each task is one red-green-refactor cycle with the named smoke test. Red means the test fails on an assertion. Green means `npm run build`, `npm run lint` and the full `scripts/run-smoke-tests.sh` pass.

- [x] 2.1 `tasks.spec.js` › also assert the header has no `Tasks` link and no `Add` button, and Tasks is reached only through the `Settings` menu.
- [x] 2.2 `addChannelDialog.spec.js` › `it should show no notice until a handle is entered`: the `destination-notice` element exists only once the handle is non-empty.
- [x] 2.3 `addChannelDialog.spec.js` › `it should state the video limit and destination for a handle`:
  - Render `DestinationNotice` above "Advanced options".
  - Build it from `channelNoticeLead(video_limit)` and the `destination` that `LocationField` now reports.
  - Pass `showDestination={false}` and keep the advanced block mounted (`hidden`), which drops the destination box and its hints.
- [x] 2.4 `addChannelDialog.spec.js` › `it should update the notice from the advanced options`: limit 1 gives "The latest video". A renamed folder shows in the path. `channelNoticeLead` handles the singular, plural and invalid-limit cases.
- [x] 2.5 `addChannelDialog.spec.js` › `it should expand advanced options from the change action`: `[change]` sets `advancedOpen`, and `Folder name` is visible with the derived value.
- [ ] 2.6 `mobile-sidebar.spec.js` › `it should close the drawer and open the add channel dialog`: at 390px, "Add channel" closes the drawer and shows the dialog.
- [ ] 2.7 `channel.spec.js` › lifecycle:
  - Before submitting, the notice names `/channels/<slug>`.
  - After creation, reopening with the same handle shows the error notice ("already used by"), and `Create Channel` is disabled.
  - `LocationField` reports `occupiedBy`, and the notice switches to its error style.

## 3. Infrastructure adapters (TDD)

None. The change is frontend only.

## 4. Verification

- [ ] 4.1 `playlist.spec.js` and `addDialogLocation.spec.js` pass unchanged apart from the opener. `scripts/run-smoke-tests.sh` is fully green.
- [ ] 4.2 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass (no Rust changes expected).
- [ ] 4.3 Manual check with `scripts/run-local.sh` at desktop width and at about 375px:
  - The header shows only the logo (plus the menu button on mobile) and the gear, and the gear menu leads to Tasks.
  - "Add channel" and "Add playlist" sit under their headings, including when a section is empty or collapsed.
  - The channel dialog shows only the handle field until something is typed. Then the notice appears above "Advanced options" and follows limit and folder edits, and `[change]` expands the section.
  - A conflicting folder turns the notice into an error and disables submit.
  - The playlist dialog behaves as before.
