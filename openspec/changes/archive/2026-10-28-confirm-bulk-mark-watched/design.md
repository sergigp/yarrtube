# Design

Frontend only (`web/`). Follows the existing delete-confirmation pattern: whoever owns the menu's row or header owns the `ConfirmDialog`.

## Files

- `web/src/lib/markWatchedConfirmation.ts` — new; pure rule "confirm above 1 unwatched" + dialog copy, unit-testable.
- `web/src/lib/markWatchedConfirmation.test.ts` — new; its unit tests.
- `web/src/components/ConfirmDialog.tsx` — add `pendingLabel` (today hard-coded "Deleting…").
- `web/src/components/DetailHeader.tsx` — "Mark all watched" opens the confirmation or marks directly, using its existing `unwatchedCount` prop (channel, playlist, category views).
- `web/src/components/Sidebar.tsx` — `SidebarSection` owns a second `ConfirmDialog` (next to the delete one) for channel/playlist rows, using `item.unwatched_count`.
- `web/src/api/types.ts` — fix stale comment claiming playlists report no unwatched count.
- `web/src/components/{ConfirmDialog,DetailHeader,Sidebar,CategoryDetail}.test.tsx` — tests.

`EntryActionsMenu` is unchanged: it still calls `onMarkWatched`, which the sidebar now resolves either by opening the dialog or by marking.

## Types & Signatures

```ts
// lib/markWatchedConfirmation.ts
export interface MarkWatchedConfirmation {
  title: string        // `Mark ${n} videos in "${name}" watched?`
  description: string  // "This can't be undone in bulk: videos can only be marked unwatched one by one."
}
/** null when 0 or 1 videos are unwatched: mark without asking. */
export function markWatchedConfirmation(
  name: string,
  unwatchedCount: number,
): MarkWatchedConfirmation | null

// ConfirmDialog.tsx
interface ConfirmDialogProps {
  // ...existing
  /** Confirm button text while onConfirm is pending. Default "Deleting…". */
  pendingLabel?: string
}

// Sidebar.tsx, inside SidebarSection
const [pendingMarkWatched, setPendingMarkWatched] = useState<LibraryItem | null>(null)
```

`unwatchedCount` passed to the rule is always the raw DTO `unwatched_count`, never `shownUnwatchedCount(item)` (which hides the count when `hide_unwatched_count`).

## Call Stack

Header (channel / playlist / category view):
```
DetailHeader "Mark all watched".onSelect()
  markWatchedConfirmation(name, unwatchedCount ?? 0)
    null    -> alertOnFailure(onMarkWatched, ...)              (as today)
    {..}    -> setConfirmingMarkWatched(true)
               ConfirmDialog(title, description, confirmLabel "Mark watched",
                             pendingLabel "Marking…", onConfirm = onMarkWatched)
                 -> error shown inline on failure, closes on success
```

Sidebar row (channel / playlist):
```
EntryActionsMenu "Mark all watched" -> onMarkWatched()
  SidebarSection: markWatchedConfirmation(item.name, item.unwatched_count ?? 0)
    null    -> onMarkWatched(item.id)        (EntryActionsMenu alerts on failure, as today)
    {..}    -> setPendingMarkWatched(item)   (resolves immediately)
               ConfirmDialog(..., onConfirm = () => onMarkWatched(item.id))
```

## Test Plan

Behaviour tests (Vitest, `mockApi` for the network only), in order:

1. `markWatchedConfirmation` — returns null for 0 and for 1 unwatched.
2. `markWatchedConfirmation` — for 23 unwatched in "Foo", returns title `Mark 23 videos in "Foo" watched?` and the can't-undo description.
3. `ConfirmDialog` — shows `pendingLabel` on the confirm button while `onConfirm` is pending.
4. `DetailHeader` — with 23 unwatched, "Mark all watched" opens the confirmation and does not mark; confirming calls `onMarkWatched` and closes.
5. `DetailHeader` — cancelling the confirmation never calls `onMarkWatched`.
6. `DetailHeader` — with 1 unwatched, "Mark all watched" marks without a dialog.
7. `DetailHeader` — when marking fails after confirming, the dialog stays open and shows the error.
8. `CategoryDetail` — "Mark all watched" on a category with 3 unwatched asks for confirmation before `POST .../watched` is sent.
9. `Sidebar` — channel row with 2+ unwatched: menu "Mark all watched" opens the confirmation; confirming sends `POST /api/channels/:id/watched` (update existing tests at `Sidebar.test.tsx:469`/`:489` to confirm).
10. `Sidebar` — playlist row with 1 unwatched marks without a dialog.
11. `Sidebar` — playlist with hidden count and 4 unwatched still asks, saying 4 videos.

No infrastructure tests: no adapter changes.
