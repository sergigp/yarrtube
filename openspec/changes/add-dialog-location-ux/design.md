# Design

Frontend-only change in `web/`. The Rust TDD conventions map onto the Vitest suite: behaviour tests are component tests driving the dialogs through `mockApi`, infrastructure-style tests are unit tests of the new pure module.

## Files

- `web/src/lib/saveLocations.ts` (+ `.test.ts`) — new: pure derivation/ordering of suggested parent folders and the remembered-parent localStorage read/write; in `lib/` so it is unit-testable as plain functions.
- `web/src/hooks/useSaveLocation.ts` — new: owns the location state (parent, folder name, candidates, browser open/closed, composed `LocationValue`) so the dialog can render the "Save to" list and the Advanced folder-name field in different places from one source of truth; replaces `LocationField`'s internal state + `onChange` effect contract.
- `web/src/components/SaveToField.tsx` (+ `.test.tsx`) — new: the "Save to" single-choice list with the inline folder browser behind "Choose another folder…".
- `web/src/components/FolderBrowser.tsx` — new: the breadcrumb/entries/new-folder browser, extracted unchanged in behavior from `LocationField`.
- `web/src/components/FolderNameField.tsx` — new: the folder-name input that now lives under Advanced options, shared by both dialogs.
- `web/src/components/LocationField.tsx` (+ `.test.tsx`) — deleted: superseded by the three components above; its tests migrate to `SaveToField.test.tsx` / dialog tests.
- `web/src/components/DestinationNotice.tsx` — changed: drops the `onChange` prop and "change" link; confirmation only.
- `web/src/components/AddPlaylistDialog.tsx` (+ `.test.tsx`) — changed: new layout (URL → Save to → notice → Advanced → submit), `useSaveLocation`, remembered parent written on successful submit, wider dialog (`sm:max-w-lg`), looser vertical rhythm (`gap-5`/`py` bump) so the form breathes.
- `web/src/components/AddChannelDialog.tsx` (+ `.test.tsx`) — changed: same treatment, channel mode.

## Types & Signatures

```ts
// web/src/lib/saveLocations.ts
export type SaveMode = 'playlist' | 'channel'

export interface SaveCandidate {
  path: string   // parent folder relative to the videos root, '' allowed only never (default parent is 'playlists'/'channels')
  count: number  // tracked items of this mode stored directly under it; 0 for the default parent when empty
}

/**
 * Default parent pinned first, then the distinct parents of `items`' paths
 * (excluding the default), ordered by max `created_at` desc; items without
 * `created_at` (channels DTO has none) fall back to count desc, then name.
 * Returns at most `cap` (default 5) candidates.
 */
export function deriveSaveCandidates(
  defaultParent: string,
  items: { path: string; created_at?: string }[],
  cap?: number,
): SaveCandidate[]

/** null when nothing remembered or storage throws. */
export function readRememberedParent(mode: SaveMode): string | null
/** Swallows storage errors. */
export function writeRememberedParent(mode: SaveMode, parent: string): void
```

```ts
// web/src/hooks/useSaveLocation.ts
export interface SaveLocation {
  parent: string
  candidates: SaveCandidate[]      // derived + a session candidate for a browser-picked folder
  selectParent: (path: string) => void
  browserOpen: boolean
  openBrowser: () => void          // "Choose another folder…"
  closeBrowser: () => void         // adds the browsed folder as the selected session candidate
  folderName: string
  setFolderName: (name: string) => void
  folderNameError: string | null
  occupied: Map<string, string>    // destination path -> occupying item name (for FolderBrowser)
  value: LocationValue             // unchanged shape: { path, destination, valid, occupiedBy }
  remember: () => void             // writeRememberedParent(mode, parent); called by the dialog on successful submit
}

export function useSaveLocation(mode: SaveMode, nameSource: string): SaveLocation
```

```ts
// components
export function SaveToField({ location }: { location: SaveLocation }): ReactElement
export function FolderBrowser(props: {
  parent: string
  occupied: Map<string, string>
  onNavigate: (path: string) => void
  onClose: () => void
}): ReactElement
export function FolderNameField({ location, id }: { location: SaveLocation; id: string }): ReactElement
// DestinationNotice: interface DestinationNoticeProps { tone: 'info' | 'error'; children: ReactNode }  (onChange removed)
```

Initial parent: `readRememberedParent(mode)` if it is among the derived candidates, else the default parent. Suggestions derive from items of the dialog's own kind only (playlists for playlist mode, channels for channel mode).

## Call Stack

Open dialog:
- `AddPlaylistDialog` → `useSaveLocation('playlist', previewTitle)` → `fetchDirectories('')` (videos root), `fetchPlaylists()` + `fetchChannels()` (occupied map; playlists also feed `deriveSaveCandidates('playlists', playlists)`) → `readRememberedParent('playlist')` → initial `parent`.

Pick a suggestion:
- `SaveToField` radio click → `location.selectParent(path)` → `value` recomputes → `PlaylistNotice` re-renders with the new destination.

Browse:
- "Choose another folder…" → `location.openBrowser()` → `FolderBrowser` → `fetchDirectories(parent)`; entry click / breadcrumb / new-folder → `onNavigate(path)` → `selectParent(path)` (live, notice follows) → close control → `location.closeBrowser()` → browsed folder shown selected in the list.

Submit:
- form submit → `createPlaylist({ playlist, path: location.value.path, quality })` → on success `location.remember()`, invalidate library, close. Channel dialog identical with `createChannel` and `video_limit`.

## Test Plan

Behaviour tests (component, `mockApi`, in dialog/`SaveToField` test files):

1. `shows the save-to list with the default parent selected on open` — list visible before any input; `playlists/` selected.
2. `suggests parent folders of tracked playlists with their counts` — playlists under `playlists/kids/*` yield a `playlists/kids` candidate showing 2 items.
3. `selects a suggested folder in one click and the notice follows` — click candidate; notice shows `<root>/playlists/kids/<slug>`.
4. `orders suggestions by most recent use and caps them` — 6 distinct parents; the 4 most recent shown after the default.
5. `preselects the remembered parent when still suggested` — seeded localStorage; candidate preselected on open.
6. `falls back to the default parent when the remembered one is stale` — remembered path with no tracked items; `playlists/` selected, no error.
7. `remembers the submitted parent for the next open` — submit into `playlists/kids`; reopen; preselected.
8. `opens the browser from choose-another and keeps the chosen folder selected` — browse to a subfolder, close; list shows it selected; notice matches.
9. `shows no change action in the notice` — info and destination-occupied error notices render without a "change" link.
10. `keeps the folder name and quality under advanced options` — collapsed by default; folder name editable after expanding; edited name sticks.
11. `suggests only parents of tracked channels in the channel dialog` — playlists-only parents absent; channel parents present.
12. `keeps a separate remembered parent per dialog` — playlist submit does not change the channel dialog's preselection.

Unit tests (`web/src/lib/saveLocations.test.ts`):

13. `derives distinct parents with counts from item paths`.
14. `pins the default parent first even with zero occupants`.
15. `orders by newest created_at, then count, then name, and applies the cap`.
16. `reads null and writes without throwing when storage is unavailable`.

Verification: `npm run check`, plus a manual pass with `scripts/run-local.sh` for the visual work (dialog width, spacing, no horizontal clipping with long "in use by" labels — jsdom cannot assert layout).
