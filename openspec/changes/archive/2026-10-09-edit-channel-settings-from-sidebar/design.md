## Files

- `web/src/hooks/useSaveChannelSettings.ts` (new): the save flow (PATCH, then sync on a limit change, alert on sync failure), moved out of `ChannelDetail` so the sidebar and the page header share it.
- `web/src/components/ChannelDetail.tsx`: uses `useSaveChannelSettings` instead of its inline save.
- `web/src/components/Sidebar.tsx`: channel rows pass `onEditRequest`; `Sidebar` owns the editing channel id and renders one `EditChannelDialog`.
- `web/src/components/EditChannelDialog.tsx`: title "Edit {name} settings" (name moves to a screen-reader-only description); quality note while the quality differs.
- `web/src/components/EntryActionsMenu.tsx`: `onEditRequest` doc no longer says "page header only".
- `web/src/components/Sidebar.test.tsx`, `EditChannelDialog.test.tsx`, `ChannelDetail.test.tsx`: new/updated tests.
- `smoke-tests/tests/channel.spec.js`: dialog matched by `/^Edit .+ settings$/` (the real channel name comes from YouTube).

## Types & Signatures

```ts
// hooks/useSaveChannelSettings.ts
export function useSaveChannelSettings(): (
  channel: ChannelListItem,
  changes: UpdateChannelRequest,
) => Promise<void>

// Sidebar.tsx (internal props)
interface SidebarSectionProps { /* … */ onEditRequest?: (id: string) => void }
interface SidebarRowProps     { /* … */ onEditRequest?: (() => void) | undefined }
interface SidebarRowMenuProps { /* … */ onEditRequest?: (() => void) | undefined }
// Sidebar state
const [editingChannelId, setEditingChannelId] = useState<string | null>(null)
```

## Call Stack

Sidebar edit:
1. `SidebarRowMenu` → `EntryActionsMenu` "Edit settings" → `onEditRequest()` → `SidebarSection` → `setEditingChannelId(item.id)`
2. `Sidebar` resolves `editingChannel` from `useChannels()` data → `<EditChannelDialog channel={editingChannel} onSave={(changes) => saveChannelSettings(editingChannel, changes)} />`
3. `saveChannelSettings(channel, changes)` → `updateChannelSettings(channel.id, changes)` → `PATCH /api/channels/{id}`; if `changes.video_limit !== undefined` → `refreshing(() => reconcileChannel(channel.id))()` → `POST /api/channels/{id}/reconcile`; failure → `window.alert(...)`

Page header edit: `ChannelDetail` → same `EditChannelDialog` → same `saveChannelSettings(channel, changes)`.

## Test Plan

Behaviour (Vitest, `mockApi`):
1. `EditChannelDialog`: opens titled "Edit Veritasium settings", prefilled.
2. `EditChannelDialog`: notes that a changed quality applies to new videos only.
3. `EditChannelDialog`: shows no quality note while the quality is unchanged.
4. `Sidebar`: a channel row menu lists Sync, Mark all watched, Edit settings, Delete.
5. `Sidebar`: edits a channel's quality from its row menu (PATCH `{ quality: 'low' }`, no reconcile, dialog closes).
6. `Sidebar`: syncs a channel after its video limit changes from its row menu.
7. `Sidebar`: playlist rows offer no Edit settings.
8. `ChannelDetail`: existing edit tests pass with the new dialog name.

Infrastructure: none.
