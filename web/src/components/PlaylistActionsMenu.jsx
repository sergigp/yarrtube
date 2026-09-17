import { useState } from 'react'
import { ConfirmDialog } from './ConfirmDialog'
import { deletePlaylist, reconcilePlaylist } from '../api'
import { Button } from '@/components/ui/button'

export function PlaylistActionsMenu({ playlist, onDeleted }) {
  const [confirmOpen, setConfirmOpen] = useState(false)

  return (
    <>
      <div className="flex shrink-0 gap-2">
        <Button
          variant="outline"
          onClick={async () => {
            try {
              await reconcilePlaylist(playlist.id)
            } catch (err) {
              window.alert(`Failed to sync "${playlist.name}": ${err.message}`)
            }
          }}
        >
          Sync
        </Button>
        <Button variant="destructive" onClick={() => setConfirmOpen(true)}>
          Delete
        </Button>
      </div>
      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={`Delete "${playlist.name}"?`}
        description="This removes the playlist from tracking, along with its video records and downloaded files."
        onConfirm={async () => {
          await deletePlaylist(playlist.id)
          onDeleted?.()
        }}
      />
    </>
  )
}
