import type { SaveMode } from '@/lib/saveLocations'

interface MoveSourceDialogProps {
  kind: SaveMode
  name: string
  /** The source's current path, relative to the videos root. */
  currentPath: string
  /** Sends the move; rejects to keep the dialog open with the error. */
  onMove: (path: string) => Promise<unknown>
  onClose: () => void
}

export function MoveSourceDialog(_props: MoveSourceDialogProps) {
  return <></>
}
