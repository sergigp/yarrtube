import { useState, type FormEvent } from 'react'
import { movePlaylist, type UpdatePlaylistRequest } from '@/api/client'
import type { PlaylistListItem } from '@/api/types'
import { intervalForm, intervalMinutes, type IntervalForm } from '@/lib/reconcileInterval'
import { useSourceCategoriesField } from '@/hooks/useSourceCategoriesField'
import { CategoryPicker } from './CategoryPicker'
import { LocationLine } from './LocationLine'
import { MoveSourceDialog } from './MoveSourceDialog'
import { ReconcileIntervalField } from './ReconcileIntervalField'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'

interface EditPlaylistDialogProps {
  playlist: PlaylistListItem
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Sends the changes; rejects to keep the dialog open with the error. */
  onSave: (changes: UpdatePlaylistRequest) => Promise<void>
}

export function EditPlaylistDialog({
  playlist,
  open,
  onOpenChange,
  onSave,
}: EditPlaylistDialogProps) {
  const [moving, setMoving] = useState(false)
  if (moving) {
    return (
      <MoveSourceDialog
        kind="playlist"
        name={playlist.name}
        currentPath={playlist.path}
        onMove={(path) => movePlaylist(playlist.id, path)}
        onClose={() => {
          setMoving(false)
          onOpenChange(false)
        }}
      />
    )
  }
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {/* Focus the first setting, not the Location line's "Move…" above it. */}
      <DialogContent
        className="gap-5 p-6 sm:max-w-md"
        onOpenAutoFocus={(event) => {
          event.preventDefault()
          document.getElementById('edit-playlist-interval')?.focus()
        }}
      >
        <DialogTitle>Edit {playlist.name} settings</DialogTitle>
        <DialogDescription className="sr-only">
          Change how often {playlist.name} is checked for new videos.
        </DialogDescription>
        {/* The form unmounts with the dialog, so every open starts from the
            playlist's current settings. */}
        <EditPlaylistForm
          playlist={playlist}
          onSave={onSave}
          onClose={() => onOpenChange(false)}
          onMoveRequest={() => setMoving(true)}
        />
      </DialogContent>
    </Dialog>
  )
}

interface EditPlaylistFormProps {
  playlist: PlaylistListItem
  onSave: (changes: UpdatePlaylistRequest) => Promise<void>
  onClose: () => void
  /** Leaves the form unsaved for the move dialog. */
  onMoveRequest: () => void
}

function EditPlaylistForm({ playlist, onSave, onClose, onMoveRequest }: EditPlaylistFormProps) {
  const [interval, changeInterval] = useState<IntervalForm>(
    intervalForm(playlist.reconcile_interval_minutes),
  )

  const [error, setError] = useState<Error | null>(null)
  const [submitting, setSubmitting] = useState(false)
  const categories = useSourceCategoriesField({ kind: 'playlist', id: playlist.id })

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const minutes = intervalMinutes(interval)
    if (minutes === null) return
    const changesInterval = minutes !== playlist.reconcile_interval_minutes
    if (!changesInterval && !categories.changed) {
      onClose()
      return
    }
    setSubmitting(true)
    setError(null)
    try {
      if (changesInterval) {
        await onSave({ reconcile_interval_minutes: minutes })
      }
      await categories.save()
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <form className="flex min-w-0 flex-col gap-5" onSubmit={handleSubmit}>
      <LocationLine
        path={playlist.path}
        onMove={onMoveRequest}
        moveBlocked={playlist.work_in_progress}
      />
      <ReconcileIntervalField id="edit-playlist-interval" value={interval} onChange={changeInterval} />
      <CategoryPicker value={categories.value} onChange={categories.onChange} />

      {error && <p className="text-sm text-destructive">{error.message}</p>}
      <Button type="submit" disabled={submitting} className="self-start">
        {submitting ? 'Saving…' : 'Save'}
      </Button>
    </form>
  )
}
