import { Button } from '@/components/ui/button'
import { unsavedChangesLabel } from '@/lib/aiChanges'

/**
 * The bar stuck to the bottom of the viewport while there are unsaved
 * changes: how many and which, why the last save failed, and Discard / Save.
 */
export function SaveBar({
  changes,
  error,
  saving,
  onDiscard,
  onSave,
}: {
  changes: string[]
  error: string | null
  saving: boolean
  onDiscard: () => void
  onSave: () => void
}) {
  if (changes.length === 0) {
    return null
  }
  return (
    <div
      role="region"
      aria-label="Unsaved changes"
      className="sticky bottom-0 z-10 -mx-4 border-t border-border bg-background/95 px-4 py-3 shadow-[0_-4px_16px_rgba(0,0,0,0.05)] backdrop-blur"
    >
      <div className="flex flex-wrap items-center justify-end gap-3">
        <div className="mr-auto flex flex-col text-sm">
          <p className="text-foreground">
            <span className="font-semibold">{unsavedChangesLabel(changes.length)}</span>
            <span className="text-muted-foreground"> · {changes.join(', ')}</span>
          </p>
          {error && (
            <p role="alert" className="text-destructive">
              {error}
            </p>
          )}
        </div>
        <Button variant="outline" disabled={saving} onClick={onDiscard}>
          Discard
        </Button>
        <Button disabled={saving} onClick={onSave}>
          Save changes
        </Button>
      </div>
    </div>
  )
}
