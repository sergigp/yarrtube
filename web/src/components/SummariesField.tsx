import { Label } from '@/components/ui/label'

interface SummariesFieldProps {
  id: string
  checked: boolean
  onChange: (checked: boolean) => void
  /** Called when the "AI settings" link is followed, to close the dialog. */
  onNavigateAway: () => void
}

/** The "Summarize videos" checkbox of the add and edit source dialogs. */
export function SummariesField({ id, checked, onChange }: SummariesFieldProps) {
  const hintId = `${id}-hint`
  return (
    <div className="flex items-start gap-2">
      <input
        id={id}
        type="checkbox"
        className="mt-0.5 size-4 shrink-0 accent-primary"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        aria-describedby={hintId}
      />
      <div className="flex flex-col gap-1">
        <Label htmlFor={id}>Summarize videos</Label>
        <p id={hintId} className="text-xs text-muted-foreground">
          Generate an AI summary of each new video.
        </p>
      </div>
    </div>
  )
}
