import { Link } from 'react-router-dom'
import { useSummariesAvailable } from '@/api/queries'
import { Label } from '@/components/ui/label'

/** An AI option of a source, as the dialog holds it. */
interface SourceAiOption {
  checked: boolean
  onChange: (checked: boolean) => void
  /** The source's stored value: when true the box stays enabled so it can be turned off. */
  enabledOnSource?: boolean
}

interface SourceAiSectionProps {
  /** Prefix of the checkboxes' ids, such as "add-channel". */
  idPrefix: string
  summaries: SourceAiOption
  /** Called when the "AI settings" link is followed, to close the dialog. */
  onNavigateAway: () => void
}

/**
 * The "Summarize videos" checkbox of the add and edit source dialogs. Only
 * usable while AI and summaries are on, or to turn off summaries a source
 * already has; otherwise it is disabled and points to the AI settings.
 */
export function SourceAiSection({ idPrefix, summaries, onNavigateAway }: SourceAiSectionProps) {
  const { checked, onChange, enabledOnSource = false } = summaries
  const id = `${idPrefix}-summaries`
  const available = useSummariesAvailable()
  const hintId = `${id}-hint`
  return (
    <div className="flex items-start gap-2">
      <input
        id={id}
        type="checkbox"
        className="mt-0.5 size-4 shrink-0 accent-primary"
        checked={checked}
        disabled={!available && !enabledOnSource}
        onChange={(event) => onChange(event.target.checked)}
        aria-describedby={hintId}
      />
      <div className="flex flex-col gap-1">
        <Label htmlFor={id}>Summarize videos</Label>
        <p id={hintId} className="text-xs text-muted-foreground">
          {available ? (
            'Generate and show an AI summary of each video.'
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
