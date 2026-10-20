import { useState, type ReactNode } from 'react'
import { RotateCw } from 'lucide-react'
import { ConfirmDialog } from './ConfirmDialog'
import { EditableTitle } from './EditableTitle'
import { EntryActionsMenu } from './EntryActionsMenu'
import { Thumbnail } from './Thumbnail'
import { Button } from '@/components/ui/button'
import { errorMessage } from '@/lib/errorMessage'

function summaryFor(
  videos: readonly unknown[] | undefined,
  unwatchedCount: number | undefined,
  prefix: string | undefined,
): string | null {
  if (!videos) {
    return null
  }
  const parts = [`${videos.length} ${videos.length === 1 ? 'video' : 'videos'}`]
  if (prefix) {
    parts.unshift(prefix)
  }
  if (unwatchedCount !== undefined && unwatchedCount > 0) {
    parts.push(`${unwatchedCount} unwatched`)
  }
  return parts.join(' · ')
}

interface DetailHeaderProps {
  name: string
  avatarSrc?: string | null
  showAvatar?: boolean
  videos: readonly unknown[] | undefined
  unwatchedCount?: number
  /** Shown before the video counts, e.g. "2 sources". */
  summaryPrefix?: string
  /** Shows the Sync control; categories have none. */
  onSync?: () => Promise<void>
  /** Shows a "Rename" pencil that edits the name in place. */
  onRename?: (name: string) => Promise<void>
  /** Items listed first in the actions menu. */
  menuItems?: ReactNode
  onMarkWatched?: () => Promise<void>
  /** Playlists only: whether the playlist is excluded from home. */
  excludedFromHome?: boolean
  onSetExcludedFromHome?: (excluded: boolean) => Promise<void>
  /** Playlists only: whether the playlist's unwatched count is hidden. */
  unwatchedCountHidden?: boolean
  onSetUnwatchedCountHidden?: (hidden: boolean) => Promise<void>
  /** Opens the edit channel or playlist dialog, which the caller owns. */
  onEditRequest?: () => void
  onDelete: () => Promise<void>
  deleteDescription: string
}

/**
 * Title block at the top of a channel/playlist page: optional avatar, the
 * name, a video/unwatched summary and the page-level actions (sync, mark all
 * watched when supported, delete behind a confirmation). Action labels
 * collapse to icons on narrow screens.
 */
export function DetailHeader({
  name,
  avatarSrc,
  showAvatar,
  videos,
  unwatchedCount,
  summaryPrefix,
  onSync,
  onRename,
  menuItems,
  onMarkWatched,
  excludedFromHome,
  onSetExcludedFromHome,
  unwatchedCountHidden,
  onSetUnwatchedCountHidden,
  onEditRequest,
  onDelete,
  deleteDescription,
}: DetailHeaderProps) {
  const [syncing, setSyncing] = useState(false)
  const [confirmingDelete, setConfirmingDelete] = useState(false)
  const summary = summaryFor(videos, unwatchedCount, summaryPrefix)

  return (
    <div className="mb-4 flex items-center gap-3 md:mb-6">
      {showAvatar && (
        <Thumbnail
          src={avatarSrc}
          className="size-10 shrink-0 rounded-full object-cover md:size-12"
        />
      )}
      <div className="flex min-w-0 flex-1 flex-col">
        {onRename ? (
          <EditableTitle name={name} onRename={onRename} />
        ) : (
          <h2 className="truncate font-heading text-lg font-semibold text-foreground md:text-xl">
            {name}
          </h2>
        )}
        {summary && <p className="truncate text-xs text-muted-foreground md:text-sm">{summary}</p>}
      </div>
      <div className="flex shrink-0 items-center gap-2">
        {onSync && (
          <Button
            variant="outline"
            size="sm"
            disabled={syncing}
            aria-label="Sync"
            onClick={async () => {
              setSyncing(true)
              try {
                await onSync()
              } catch (err) {
                window.alert(`Failed to sync "${name}": ${errorMessage(err)}`)
              } finally {
                setSyncing(false)
              }
            }}
          >
            <RotateCw className={syncing ? 'animate-spin' : undefined} />
            <span className="hidden sm:inline">Sync</span>
          </Button>
        )}
        {/* Labelled apart from the sidebar row's "Actions for <name>", which is
            on screen at the same time. */}
        <EntryActionsMenu
          name={name}
          label={`More actions for ${name}`}
          leadingItems={menuItems}
          onMarkWatched={onMarkWatched}
          excludedFromHome={excludedFromHome}
          onSetExcludedFromHome={onSetExcludedFromHome}
          unwatchedCountHidden={unwatchedCountHidden}
          onSetUnwatchedCountHidden={onSetUnwatchedCountHidden}
          onEditRequest={onEditRequest}
          onDeleteRequest={() => setConfirmingDelete(true)}
        />
      </div>

      <ConfirmDialog
        open={confirmingDelete}
        onOpenChange={setConfirmingDelete}
        title={`Delete "${name}"?`}
        description={deleteDescription}
        onConfirm={onDelete}
      />
    </div>
  )
}
