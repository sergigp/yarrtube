import { Button } from '@/components/ui/button'

/** An edit dialog's read-only "Location" line, with "Move…" to change it. */
export function LocationLine({ path, onMove }: { path: string; onMove: () => void }) {
  return (
    <div className="flex min-w-0 flex-col gap-1.5">
      <p className="text-sm leading-none font-medium">Location</p>
      <div className="flex min-w-0 items-center gap-2">
        <code className="min-w-0 flex-1 truncate font-mono text-xs text-foreground/80">{path}</code>
        <Button type="button" variant="outline" size="sm" onClick={onMove}>
          Move…
        </Button>
      </div>
    </div>
  )
}
