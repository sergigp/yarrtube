import { useState, type FormEvent } from 'react'
import { useInvalidateLibrary } from '@/api/queries'
import { useSaveLocation, type LocationValue } from '@/hooks/useSaveLocation'
import type { SaveMode } from '@/lib/saveLocations'
import { DestinationNotice, DestinationPath } from './DestinationNotice'
import { FolderNameField } from './FolderNameField'
import { SaveToField } from './SaveToField'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'

interface MoveSourceDialogProps {
  kind: SaveMode
  name: string
  /** The source's current path, relative to the videos root. */
  currentPath: string
  /** Sends the move; rejects to keep the dialog open with the error. */
  onMove: (path: string) => Promise<unknown>
  onClose: () => void
}

/** Picks a new location for a tracked playlist or channel and moves it there. */
export function MoveSourceDialog({ kind, name, currentPath, onMove, onClose }: MoveSourceDialogProps) {
  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      {/* Capped like the add dialogs, so an open folder browser can't push
          the buttons off a short viewport. */}
      <DialogContent
        className="max-h-[calc(100dvh-2rem)] gap-5 overflow-x-hidden overflow-y-auto p-6 sm:max-w-lg"
        onPointerDownOutside={(event) => event.preventDefault()}
        onInteractOutside={(event) => event.preventDefault()}
      >
        <DialogTitle>Move {name}</DialogTitle>
        <DialogDescription className="sr-only">
          Choose the folder {name} and its downloaded videos move to.
        </DialogDescription>
        <MoveSourceForm
          kind={kind}
          name={name}
          currentPath={currentPath}
          onMove={onMove}
          onClose={onClose}
        />
      </DialogContent>
    </Dialog>
  )
}

function MoveNotice({ location }: { location: LocationValue }) {
  const destination = <DestinationPath>{location.destination}</DestinationPath>
  if (location.occupiedBy) {
    return (
      <DestinationNotice tone="error">
        {destination} is already used by {location.occupiedBy}. Choose a different folder.
      </DestinationNotice>
    )
  }
  if (location.destination) {
    return <DestinationNotice tone="info">Videos and their files move to {destination}</DestinationNotice>
  }
  return null
}

function MoveSourceForm({ kind, name, currentPath, onMove, onClose }: MoveSourceDialogProps) {
  const location = useSaveLocation(kind, name, currentPath)
  const invalidateLibrary = useInvalidateLibrary()
  const [error, setError] = useState<Error | null>(null)
  const [submitting, setSubmitting] = useState(false)
  const canSubmit = location.value.valid && location.value.path !== currentPath && !submitting

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!canSubmit) {
      return
    }
    setSubmitting(true)
    setError(null)
    try {
      await onMove(location.value.path)
      invalidateLibrary()
      onClose()
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <form className="flex min-w-0 flex-col gap-5" onSubmit={handleSubmit}>
      <SaveToField location={location} stagedNote="Does not exist yet — created by the move." />
      <FolderNameField location={location} id="move-source-folder-name" />
      <MoveNotice location={location.value} />

      {error && <p className="text-sm text-destructive">{error.message}</p>}
      <div className="flex gap-2">
        <Button type="submit" disabled={!canSubmit}>
          {submitting ? 'Moving…' : 'Move'}
        </Button>
        <Button type="button" variant="outline" onClick={onClose}>
          Cancel
        </Button>
      </div>
    </form>
  )
}
