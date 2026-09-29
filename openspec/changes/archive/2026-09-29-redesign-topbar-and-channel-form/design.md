## Files

**Frontend**
- `web/src/App.jsx`:
  - The header drops the `Tasks` and `Add` buttons and renders `<SettingsMenu />`.
  - It holds `addDialog` state (`'channel' | 'playlist' | null`), passes `onAddChannel`/`onAddPlaylist` to `Sidebar` and renders both dialogs.
- `web/src/components/SettingsMenu.jsx` (new): gear icon button (`aria-label="Settings"`) that opens a `DropdownMenu` with one `Tasks` item linking to `/tasks`.
- `web/src/components/Sidebar.jsx`:
  - `Sidebar` takes `onAddChannel`/`onAddPlaylist`.
  - `SidebarSection` renders an add button (`+` icon and label) between the heading and the list, shown in every state (loading, error, empty, collapsed).
  - Clicking it calls `onClose()`, then `onAdd()`.
- `web/src/components/AddChannelDialog.jsx` (new), the channel form:
  - Controls in order: handle field, `DestinationNotice`, "Advanced options" (location, quality, limit), submit.
  - "Advanced options" stays mounted and is toggled with `hidden`, so the `LocationField` state and its reported destination survive collapsing.
- `web/src/components/AddPlaylistDialog.jsx` (new): the playlist half of today's `AddDialog`, without the tabs. Behaviour is unchanged.
- `web/src/components/AddDialog.jsx`: deleted.
- `web/src/components/VideoQualityField.jsx` (new): `VideoQualityField` moved out of `AddDialog.jsx`, shared by both dialogs.
- `web/src/components/LocationField.jsx`:
  - `onChange` also reports the absolute destination and the occupant.
  - New `showDestination` prop (default `true`). When it is `false`, the "Download destination" box, its creation hints and its conflict line are not rendered.
- `web/src/channelNotice.js` (new): a pure function that builds the notice sentence from the limit, so the pluralisation and the invalid-limit fallback live in one place.

**Smoke tests**
- `smoke-tests/helpers/addDialog.js`:
  - `openAddDialog` is replaced by `openAddChannelDialog`/`openAddPlaylistDialog`, which click the sidebar entries.
  - The tab clicks are removed.
  - `fillChannel` opens "Advanced options" before setting the location.
  - New `destinationNotice(dialog)` helper.
- `smoke-tests/tests/channel.spec.js`, `playlist.spec.js`, `addDialogLocation.spec.js`: use the new openers.
- `smoke-tests/tests/tasks.spec.js`: reach Tasks through the settings menu.
- `smoke-tests/tests/mobile-sidebar.spec.js`: new add-from-drawer case.
- `smoke-tests/tests/addChannelDialog.spec.js` (new): notice cases that don't need YouTube.

## Types & Signatures

```js
// web/src/App.jsx
const [addDialog, setAddDialog] = useState(null) // 'channel' | 'playlist' | null
<Sidebar open onClose onAddChannel={() => setAddDialog('channel')} onAddPlaylist={() => setAddDialog('playlist')} />
<AddChannelDialog open={addDialog === 'channel'} onOpenChange={(open) => !open && setAddDialog(null)} />
<AddPlaylistDialog open={addDialog === 'playlist'} onOpenChange={(open) => !open && setAddDialog(null)} />
```

```js
// web/src/components/SettingsMenu.jsx
export function SettingsMenu()
```

```js
// web/src/components/Sidebar.jsx
export function Sidebar({ open, onClose, onAddChannel, onAddPlaylist })
function SidebarSection({ title, addLabel, onAdd, ...existing })
```

```js
// web/src/components/AddChannelDialog.jsx
export function AddChannelDialog({ open, onOpenChange })
function DestinationNotice({ videoLimit, location, onChange }) // hidden by the caller while the handle is empty

// web/src/components/AddPlaylistDialog.jsx
export function AddPlaylistDialog({ open, onOpenChange })

// web/src/components/VideoQualityField.jsx
export function VideoQualityField({ id, value, onChange })
```

```js
// web/src/components/LocationField.jsx
export function LocationField({ mode, nameSource, onChange, showDestination = true })
// onChange payload (was { path, valid }):
// {
//   path,          // relative destination, sent to the API
//   destination,   // `${root}/${path}`, or '' while the folder name is invalid
//   valid,         // !folderNameError && !occupiedBy
//   occupiedBy,    // name of the playlist/channel using `path`, or undefined
// }
```

```js
// web/src/channelNotice.js
// "The latest 3 videos from this channel will be downloaded to"
// "The latest video from this channel will be downloaded to"  (limit 1)
// "Videos from this channel will be downloaded to"             (limit not a whole number in 1..1000)
export function channelNoticeLead(videoLimit) // -> string
```

## Call Stack

**Open an add dialog from the sidebar**
1. `SidebarSection` add button `onClick` → `onClose()` (mobile drawer; no-op on desktop) → `onAdd()`
2. → `App.setAddDialog('channel' | 'playlist')` → the matching dialog renders `open`

**Add channel dialog**
1. The handle field `onChange` → `channelForm.channel`
2. → `<LocationField mode="channel" nameSource={deriveChannelPathSegment(channel)} showDestination={false} onChange={setLocation} />`. It is mounted inside the `hidden` advanced block and reports `{ path, destination, valid, occupiedBy }`.
3. The notice renders when `channel.trim() && location.destination`:
   - `occupiedBy` set: error style, "`{destination}` is already used by `{occupiedBy}`. Choose a different folder." plus `[change]`
   - otherwise: info style, `channelNoticeLead(video_limit)` + `{destination}` + `[change]`
4. `[change]` → `setAdvancedOpen(true)`
5. Submit (disabled while `!location.valid || submitting`) → `createChannel({ channel, quality, video_limit: Number(video_limit), path: location.path })` → `invalidateLibrary()` → reset → `onOpenChange(false)`

**Add playlist dialog**: today's playlist flow. `LocationField mode="playlist"` keeps its destination box. `createPlaylist({ playlist, name, path, quality })` → `invalidateLibrary()`.

**Settings menu**: gear `DropdownMenuTrigger` → `DropdownMenuItem asChild` → `<Link to="/tasks">Tasks</Link>`

## Test Plan

The frontend has no unit test runner, so behaviour is covered by Playwright smoke tests (`scripts/run-smoke-tests.sh`) and checked with `npm run build` and `npm run lint` in `web/`. No Rust changes.

**Behaviour tests** (smoke)
1. `tasks.spec.js` › `tasks view lists the recurring yt-dlp self-update task`: it reaches Tasks through the `Settings` menu. There is no `Tasks` link or `Add` button in the header.
2. `addChannelDialog.spec.js` › `it should show no notice until a handle is entered`: the dialog opens from "Add channel" and `destinationNotice` is absent.
3. `addChannelDialog.spec.js` › `it should state the video limit and destination for a handle`: fill `@some-handle`. The notice contains "The latest 3 videos" and ends with `/channels/some-handle`, and "Advanced options" is collapsed.
4. `addChannelDialog.spec.js` › `it should update the notice from the advanced options`: set the video limit to 1 and the folder name to `renamed`. The notice reads "The latest video" and ends with `/channels/renamed`.
5. `addChannelDialog.spec.js` › `it should expand advanced options from the change action`: click `change`. The `Advanced options` toggle becomes `aria-expanded="true"` and `Folder name` is visible with value `some-handle`.
6. `mobile-sidebar.spec.js` › `it should close the drawer and open the add channel dialog`: at 390px, open the drawer and click "Add channel". The overlay is gone and the dialog is visible.
7. `channel.spec.js` › lifecycle: add through "Add channel". Before submitting, the notice names `/channels/<slug of handle>`. After creation, reopening with the same handle shows the error notice ("already used by") and `Create Channel` is disabled. Then the lifecycle continues as today.
8. `playlist.spec.js`, `addDialogLocation.spec.js`: unchanged cases, now opened through "Add playlist", with no tab click.
