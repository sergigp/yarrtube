# Design

## Context

- `DetailHeader.tsx` builds one `HeaderAction[]`. On desktop each action becomes an `IconAction` and `MoreActionsMenu` gets `actions=[]` (Delete only). Below desktop every action goes into `MoreActionsMenu`. Channel, playlist and category views all use it.
- `VideoDetail.tsx` renders the title row as `[avatar] title · PlaybackSpeedMenu · VideoActionsMenu`. `VideoActionsMenu` is shared with home cards and list rows and is only used here for "Mark as watched" (`markable = !watched && DOWNLOADED`).
- `useMarkVideoWatched` already refreshes the lists and the selected video's watched state. `useWatchProgress` refetches when a rewatch changes the watched state, so a status bound to `video.watched` follows it with no extra work.
- The main spec's "Video Detail Title Prominence" said nothing shares the title row, but the speed control and ⋮ already did. This change updates it to allow the speed control (the ⋮ goes away); the watched control goes below the meta line, not in the title row.

## Goals / Non-Goals

**Goals:** the bulk action is reachable only through a menu on every viewport; the selected video's watched state is visible and can be acted on in one click.

**Non-Goals:** a per-video "mark unwatched" action, undo for bulk marks, changes to sidebar row menus, home cards or list rows, any backend change.

## Decisions

1. **Mark the action as menu-only on desktop with a flag on `HeaderAction`.** Add `menuOnDesktop?: boolean`. On desktop, actions with the flag go to `MoreActionsMenu` (above the separator and Delete) and the rest stay `IconAction`s. Below desktop nothing changes. Only "Mark all watched" sets it. The confirm dialog, `markWatchedPrompt` and the disabled-while-marking logic stay as they are.
   - *Alternative:* a separate `menuActions` prop. Rejected: categories pass extra actions through `extraActions`, and a per-action flag keeps the order in one place.
   - Category view gets this for free, since it uses the same component.

2. **New `VideoWatchedControl` component**, placed in a row under `MetaLine` in `VideoDetail`.
   - Props: `videoId`, `title`, `watched`, `downloaded`.
   - It renders nothing when the video isn't downloaded.
   - When unwatched: an outline small `Button` with a `Check` icon reading "Mark watched". It calls `useMarkVideoWatched`, disables itself while pending, and on failure uses `window.alert` with the same wording as `VideoActionsMenu` ("Failed to mark "<title>" watched: …").
   - When watched: a muted `<span>` with `CheckCheck`/`Check` reading "Watched", plus `title="Rewatch past 10% to mark it unwatched"`. It is not a button, so it is not focusable as an action.
   - *Alternative:* placing it in the title row on desktop and below the meta line on mobile. Rejected: two placements is more code and more tests, and squeezes the title.

3. **Remove `VideoActionsMenu` from `VideoDetail`.** In the detail pane it would only offer an item that the new control already covers. The selected list row still hides its own ⋮ but keeps the space for it, so it is unchanged.

4. **The watched hint uses the native `title` attribute, not a Radix Tooltip.** A non-interactive span can't take keyboard focus, so a Tooltip wouldn't add much there. `title` is enough and is easy to test.

## Risks / Trade-offs

- [Users used to the desktop header icon now need two clicks] → acceptable, it is the intended friction; the confirmation stays as well.
- [An extra row in the detail pane adds height on mobile] → it's a single small button row. The pane's details already start collapsed on mobile.
- [The hint's "10%" is copied from domain logic] → the copy states it once, in the component. If the threshold changes in `video.rs`, the copy needs updating too.
