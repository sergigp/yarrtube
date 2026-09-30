import { useState } from 'react'
import { CheckCheck, RotateCw, Trash2 } from 'lucide-react'
import type { Video } from '@/api/types'
import { ConfirmDialog } from './ConfirmDialog'
import { Thumbnail } from './Thumbnail'
import { Button } from '@/components/ui/button'

function summaryFor(videos: Video[] | undefined, unwatchedCount: number | undefined): string | null {
  if (!videos) {
    return null
  }
  const parts = [`${videos.length} ${videos.length === 1 ? 'video' : 'videos'}`]
  if (unwatchedCount !== undefined && unwatchedCount > 0) {
    parts.push(`${unwatchedCount} unwatched`)
  }
  return parts.join(' · ')
}

interface DetailHeaderProps {
  name: string
  avatarSrc?: string | null
  showAvatar?: boolean
  videos: Video[] | undefined
  unwatchedCount?: number
  onSync: () => Promise<void>
  onMarkWatched?: () => Promise<void>
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
  onSync,
  onMarkWatched,
  onDelete,
  deleteDescription,
}: DetailHeaderProps) {
  const [syncing, setSyncing] = useState(false)
  const [markingWatched, setMarkingWatched] = useState(false)
  const [confirmingDelete, setConfirmingDelete] = useState(false)
  const summary = summaryFor(videos, unwatchedCount)

  return (
    <div className="mb-4 flex items-center gap-3 md:mb-6">
      {showAvatar && (
        <Thumbnail
          src={avatarSrc}
          className="size-10 shrink-0 rounded-full object-cover md:size-12"
        />
      )}
      <div className="flex min-w-0 flex-1 flex-col">
        <h2 className="truncate font-heading text-lg font-semibold text-foreground md:text-xl">
          {name}
        </h2>
        {summary && <p className="truncate text-xs text-muted-foreground md:text-sm">{summary}</p>}
      </div>
      <div className="flex shrink-0 items-center gap-2">
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
        {onMarkWatched && (
          <Button
            variant="outline"
            size="sm"
            disabled={markingWatched}
            aria-label="Mark all watched"
            onClick={async () => {
              setMarkingWatched(true)
              try {
                await onMarkWatched()
              } catch (err) {
                window.alert(`Failed to mark "${name}" watched: ${errorMessage(err)}`)
              } finally {
                setMarkingWatched(false)
              }
            }}
          >
            <CheckCheck />
            <span className="hidden sm:inline">Mark all watched</span>
          </Button>
        )}
        <Button
          variant="outline"
          size="sm"
          className="text-destructive hover:text-destructive"
          aria-label="Delete"
          onClick={() => setConfirmingDelete(true)}
        >
          <Trash2 />
        </Button>
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

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}
