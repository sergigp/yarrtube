import { useState } from 'react'
import { ConfirmDialog } from './ConfirmDialog'
import { deleteChannel, reconcileChannel } from '../api'

export function ChannelActionsMenu({ channel, onDeleted }) {
  const [confirmOpen, setConfirmOpen] = useState(false)

  return (
    <>
      <div className="detail-actions">
        <button
          className="secondary-button"
          onClick={async () => {
            try {
              await reconcileChannel(channel.id)
            } catch (err) {
              window.alert(`Failed to sync "${channel.name}": ${err.message}`)
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
        title={`Delete "${channel.name}"?`}
        description="This removes the channel from tracking."
        onConfirm={async () => {
          await deleteChannel(channel.id)
          onDeleted?.()
        }}
      />
    </>
  )
}
