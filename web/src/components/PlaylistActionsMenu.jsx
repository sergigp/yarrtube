import { useState } from 'react'
import { ConfirmDialog } from './ConfirmDialog'
import { deletePlaylist, reconcilePlaylist } from '../api'

export function PlaylistActionsMenu({ playlist, onDeleted }) {
  const [confirmOpen, setConfirmOpen] = useState(false)

  return (
    <>
      <div className="detail-actions">
        <button
          className="secondary-button"
          onClick={async () => {
            try {
              await reconcilePlaylist(playlist.id)
            } catch (err) {
              window.alert(`Failed to sync "${playlist.name}": ${err.message}`)
            }
          }}
        >
          Sync
        </button>
        <button className="danger-button" onClick={() => setConfirmOpen(true)}>
          Delete
        </button>
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
