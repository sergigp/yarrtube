import { useState } from 'react'
import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { ConfirmDialog } from './ConfirmDialog'
import { deleteChannel } from '../api'

export function ChannelActionsMenu({ channel, onDeleted }) {
  const [confirmOpen, setConfirmOpen] = useState(false)

  return (
    <>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger asChild>
          <button
            className="icon-button"
            aria-label={`Actions for ${channel.name}`}
            onClick={(event) => event.stopPropagation()}
          >
            ⋯
          </button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content className="dropdown-menu-content" align="end">
            <DropdownMenu.Item
              className="dropdown-menu-item dropdown-menu-item-danger"
              onSelect={() => setConfirmOpen(true)}
            >
              Delete
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
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
