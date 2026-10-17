import type { UpdatePlaylistRequest } from '@/api/client'
import type { PlaylistListItem } from '@/api/types'

interface EditPlaylistDialogProps {
  playlist: PlaylistListItem
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Sends the changes; rejects to keep the dialog open with the error. */
  onSave: (changes: UpdatePlaylistRequest) => Promise<void>
}

export function EditPlaylistDialog(_props: EditPlaylistDialogProps) {
  return null
}
