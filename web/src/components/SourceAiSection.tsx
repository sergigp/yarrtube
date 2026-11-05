import { Link } from 'react-router-dom'
import {
  useCategoriesShown,
  useCategorizationAvailable,
  useSummariesAvailable,
} from '@/api/queries'
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
  categorization: SourceAiOption
  /** Called when the "AI settings" link is followed, to close the dialog. */
  onNavigateAway: () => void
}

/**
 * The "AI" section of the add and edit source dialogs: "Summarize videos" and,
 * while categories are on, "Categorize videos".
 */
export function SourceAiSection({
  idPrefix,
  summaries,
  categorization,
  onNavigateAway,
}: SourceAiSectionProps) {
  const summariesAvailable = useSummariesAvailable()
  const categorizationAvailable = useCategorizationAvailable()
  const withCategories = useCategoriesShown()
  return (
    <fieldset className="flex min-w-0 flex-col gap-3">
      <legend className="mb-2 text-sm font-medium">AI</legend>
      <AiCheckbox
        id={`${idPrefix}-summaries`}
        label="Summarize videos"
        hint="Generate and show an AI summary of each video."
        feature="AI summaries"
        available={summariesAvailable}
        option={summaries}
        onNavigateAway={onNavigateAway}
      />
      {withCategories && (
        <AiCheckbox
          id={`${idPrefix}-categorization`}
          label="Categorize videos"
          hint="Let AI add each video to the categories it fits."
          feature="AI categorization"
          available={categorizationAvailable}
          option={categorization}
          onNavigateAway={onNavigateAway}
        />
      )}
    </fieldset>
  )
}

interface AiCheckboxProps {
  id: string
  label: string
  hint: string
  /** What to turn on in the AI settings, for the hint shown while unavailable. */
  feature: string
  available: boolean
  option: SourceAiOption
  onNavigateAway: () => void
}

/**
 * Only usable while its feature is available, or to turn it off on a source
 * that already has it; otherwise it is disabled and points to the AI settings.
 */
function AiCheckbox({
  id,
  label,
  hint,
  feature,
  available,
  option: { checked, onChange, enabledOnSource = false },
  onNavigateAway,
}: AiCheckboxProps) {
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
        <Label htmlFor={id}>{label}</Label>
        <p id={hintId} className="text-xs text-muted-foreground">
          {available ? (
            hint
          ) : (
            <>
              Turn on {feature} in{' '}
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
