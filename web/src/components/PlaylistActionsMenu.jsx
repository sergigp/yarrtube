import { useState } from 'react'
import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { ConfirmDialog } from './ConfirmDialog'
import { deletePlaylist, reconcilePlaylist } from '../api'

export function PlaylistActionsMenu({ playlist, onDeleted }) {
  const [confirmOpen, setConfirmOpen] = useState(false)

  return (
    <>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger asChild>
          <button
            className="icon-button"
            aria-label={`Actions for ${playlist.name}`}
            onClick={(event) => event.stopPropagation()}
          >
            ⋯
          </button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content className="dropdown-menu-content" align="end">
            <DropdownMenu.Item
              className="dropdown-menu-item"
              onSelect={async () => {
                try {
                  await reconcilePlaylist(playlist.id)
                } catch (err) {
                  window.alert(`Failed to reconcile "${playlist.name}": ${err.message}`)
                }
              }}
            >
              Reconcile
            </DropdownMenu.Item>
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
