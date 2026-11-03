import { Link } from 'react-router-dom'
import { useSummariesAvailable } from '@/api/queries'
import { Label } from '@/components/ui/label'

interface SummariesFieldProps {
  id: string
  checked: boolean
  onChange: (checked: boolean) => void
  /** Called when the "AI settings" link is followed, to close the dialog. */
  onNavigateAway: () => void
}

/**
 * The "Summarize videos" checkbox of the add and edit source dialogs. Only
 * usable while AI and summaries are on; otherwise it keeps its value,
 * disabled, and points to the AI settings.
 */
export function SummariesField({ id, checked, onChange, onNavigateAway }: SummariesFieldProps) {
  const available = useSummariesAvailable()
  const hintId = `${id}-hint`
  return (
    <div className="flex items-start gap-2">
      <input
        id={id}
        type="checkbox"
        className="mt-0.5 size-4 shrink-0 accent-primary"
        checked={checked}
        disabled={!available}
        onChange={(event) => onChange(event.target.checked)}
        aria-describedby={hintId}
      />
      <div className="flex flex-col gap-1">
        <Label htmlFor={id}>Summarize videos</Label>
        <p id={hintId} className="text-xs text-muted-foreground">
          {available ? (
            'Generate an AI summary of each new video.'
          ) : (
            <>
              Turn on AI summaries in{' '}
              <Link to="/ai" className="underline hover:text-foreground" onClick={onNavigateAway}>
                AI settings
              </Link>
              .
            </>
          )}
        </p>
      </div>
    </div>
  )
}
