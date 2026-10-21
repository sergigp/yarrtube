import { Button } from '@/components/ui/button'

interface LocationLineProps {
  path: string
  onMove: () => void
  /** The source still has work in progress, so a move would be refused. */
  moveBlocked?: boolean
}

/** An edit dialog's read-only "Location" line, with "Move…" to change it. */
export function LocationLine({ path, onMove, moveBlocked = false }: LocationLineProps) {
  return (
    <div className="flex min-w-0 flex-col gap-1.5">
      <p className="text-sm leading-none font-medium">Location</p>
      <div className="flex min-w-0 items-center gap-2">
        <code className="min-w-0 flex-1 truncate font-mono text-xs text-foreground/80">{path}</code>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onMove}
          disabled={moveBlocked}
          aria-describedby={moveBlocked ? 'location-move-blocked' : undefined}
        >
          Move…
        </Button>
      </div>
      {moveBlocked && (
        <p id="location-move-blocked" className="text-xs text-muted-foreground">
          Downloads in progress — you can move it once they finish.
        </p>
      )}
    </div>
  )
}
