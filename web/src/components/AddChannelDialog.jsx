import * as Dialog from '@radix-ui/react-dialog'
import { CreateChannelForm } from './CreateChannelForm'

export function AddChannelDialog({ open, onOpenChange }) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="dialog-overlay" />
        <Dialog.Content className="dialog-content">
          <Dialog.Title className="dialog-title">Add Channel</Dialog.Title>
          <CreateChannelForm onSuccess={() => onOpenChange(false)} />
          <Dialog.Close asChild>
            <button className="dialog-close" aria-label="Close">
              ×
            </button>
          </Dialog.Close>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}
