import { useState } from 'react'
import { ConfirmDialog } from './ConfirmDialog'
import { deleteChannel, reconcileChannel } from '../api'
import { Button } from '@/components/ui/button'

export function ChannelActionsMenu({ channel, onDeleted }) {
  const [confirmOpen, setConfirmOpen] = useState(false)

  return (
    <>
      <div className="flex shrink-0 gap-2">
        <Button
          variant="outline"
          onClick={async () => {
            try {
              await reconcileChannel(channel.id)
            } catch (err) {
              window.alert(`Failed to sync "${channel.name}": ${err.message}`)
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
