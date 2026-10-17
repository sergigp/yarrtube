import { useState } from 'react'
import type { UpdatePlaylistRequest } from '@/api/client'
import type { PlaylistListItem } from '@/api/types'
import { intervalForm, type IntervalForm } from '@/lib/reconcileInterval'
import { ReconcileIntervalField } from './ReconcileIntervalField'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'

interface EditPlaylistDialogProps {
  playlist: PlaylistListItem
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Sends the changes; rejects to keep the dialog open with the error. */
  onSave: (changes: UpdatePlaylistRequest) => Promise<void>
}

export function EditPlaylistDialog({ playlist, open, onOpenChange }: EditPlaylistDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="gap-5 p-6 sm:max-w-md">
        <DialogTitle>Edit {playlist.name} settings</DialogTitle>
        <DialogDescription className="sr-only">
          Change how often {playlist.name} is checked for new videos.
        </DialogDescription>
        {/* The form unmounts with the dialog, so every open starts from the
            playlist's current settings. */}
        <EditPlaylistForm playlist={playlist} />
      </DialogContent>
    </Dialog>
  )
}

interface EditPlaylistFormProps {
  playlist: PlaylistListItem
}

function EditPlaylistForm({ playlist }: EditPlaylistFormProps) {
  const [interval, changeInterval] = useState<IntervalForm>(
    intervalForm(playlist.reconcile_interval_minutes),
  )

  return (
    <form className="flex min-w-0 flex-col gap-5">
      <ReconcileIntervalField id="edit-playlist-interval" value={interval} onChange={changeInterval} />
    </form>
  )
}
