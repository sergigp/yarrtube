## 1. Walking skeleton

- [x] 1.1 Create every file and signature from design.md (`lib/playbackSpeed.ts`, `hooks/usePlaybackSpeed.ts`, `components/PlaybackSpeedMenu.tsx`), wire `usePlaybackSpeed` into `PlaylistDetail` and `ChannelDetail`, pass `playbackRate`/`onPlaybackRateChange` through `VideoDetail` and render `PlaybackSpeedMenu` in its title row. Bodies are trivial (`PLAYBACK_SPEEDS = []`, `formatPlaybackSpeed` returns `''`, hook returns `{ rate: 1, changeRate: () => {} }`, menu renders a bare trigger). No new tests. Done when `npm run typecheck` and `npm run test` pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it('offers 1x, 1.1x, 1.25x, 1.5x and 2x')` — drives `PLAYBACK_SPEEDS`.
- [x] 2.2 `it('formats a speed as its multiplier')` — drives `formatPlaybackSpeed`.
- [x] 2.3 `it('starts at normal speed')` — drives the hook's initial `rate`.
- [x] 2.4 `it('applies a chosen speed to the video element')` — drives `changeRate` setting `playbackRate`.
- [x] 2.5 `it('follows speed changes made by the native controls')` — drives the `ratechange` listener.
- [x] 2.6 `it('resets to normal speed when another video is selected')` — drives the per-video reset.
- [x] 2.7 `it('shows the current speed and marks it in the menu')` — drives the menu trigger label and checked radio item.
- [x] 2.8 `it('reports the chosen speed')` — drives `onRateChange` from radio selection.
- [x] 2.9 `it('marks no speed when the current one is not offered')` — drives off-list speed display.
- [ ] 2.10 `it('disables the speed control until the video is downloaded')` — drives `VideoDetail`'s `disabled` wiring.
- [ ] 2.11 `it('plays the selected video at the chosen speed and resets it for the next video')` — end-to-end through `PlaylistDetail`.

## 3. Infrastructure adapters (TDD)

None — frontend only, no adapters added or changed.

## 4. Verification

- [ ] 4.1 `npm run check` and `npm run build` in `web/` pass; `cargo build --release` and `cargo test --locked` pass.
- [ ] 4.2 Manual check via `scripts/run-local.sh`: in a playlist and a channel view, pick 1.1x/1.5x/2x and confirm playback speed changes, the label updates, selecting another video and reloading reset to 1x, Chrome's native speed menu is reflected in the control, and the control is disabled on a not-yet-downloaded video; check the title row layout on a mobile-width viewport.
