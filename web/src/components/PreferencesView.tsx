import { useState } from 'react'
import { Check, CircleHelp } from 'lucide-react'
import { usePreferences, useSavePreferences } from '@/api/queries'
import type { Preferences } from '@/api/types'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import { VIEW_WIDTH } from '@/lib/viewWidth'
import { errorMessage } from '@/lib/errorMessage'
import { useSavedFlash } from '@/hooks/useSavedFlash'

/** The preferences shown as switches here; the theme has its header toggle. */
type PreferenceKey = Exclude<keyof Preferences, 'theme'>

interface PreferenceGroup {
  title: string
  description?: string
  /** `needsCategories` switches are disabled while categories are off. */
  switches: {
    key: PreferenceKey
    label: string
    help: string
    needsCategories?: boolean
  }[]
}

const GROUPS: PreferenceGroup[] = [
  {
    title: 'Categories',
    switches: [
      {
        key: 'categories_enabled',
        label: 'Enable categories',
        help: 'Use categories across Yarrtube: the sidebar section, category pages, the Categories field when adding or editing a channel or playlist, and category sections on home. Turning it off only hides them: your categories and their videos are kept.',
      },
    ],
  },
  {
    title: 'Home',
    description:
      'Sections show in this order, and a video shows in one section only: turning a section off can move its videos to a section below.',
    switches: [
      {
        key: 'home_show_continue_watching',
        label: 'Show continue watching',
        help: "Videos you started in the last 7 days and haven't finished.",
      },
      {
        key: 'home_show_categories',
        label: 'Show categories',
        help: 'A "More about" section for each of up to 3 categories with unwatched videos. Needs categories enabled.',
        needsCategories: true,
      },
      {
        key: 'home_show_quick_watches',
        label: 'Show quick watches',
        help: 'Unwatched videos shorter than 15 minutes.',
      },
      {
        key: 'home_show_latest',
        label: 'Show latest videos',
        help: 'The most recently downloaded videos not shown in a section above.',
      },
    ],
  },
]

/**
 * A "?" next to a preference whose tooltip explains it. It opens on hover and
 * focus, and also on click, since a tap never hovers.
 */
function PreferenceHelp({ label, help }: { label: string; help: string }) {
  const [open, setOpen] = useState(false)
  return (
    <Tooltip open={open} onOpenChange={setOpen}>
      <TooltipTrigger asChild>
        <button
          type="button"
          aria-label={`About ${label}`}
          className="rounded-full text-muted-foreground hover:text-foreground"
          onClick={() => setOpen(!open)}
        >
          <CircleHelp className="size-4" />
        </button>
      </TooltipTrigger>
      <TooltipContent className="max-w-xs">{help}</TooltipContent>
    </Tooltip>
  )
}

/** The preferences view: a switch per preference, saved as soon as it is toggled. */
export function PreferencesView() {
  const { data: preferences, error } = usePreferences()
  const savePreferences = useSavePreferences()
  const [saveError, setSaveError] = useState<string | null>(null)
  const saved = useSavedFlash()
  // Each save sends every preference, so one at a time: a toggle while
  // another saves would send that one's old value and undo it.
  const [saving, setSaving] = useState(false)
  const toggle = async (key: PreferenceKey, value: boolean) => {
    if (!preferences || saving) {
      return
    }
    setSaving(true)
    setSaveError(null)
    saved.clear()
    try {
      await savePreferences({ ...preferences, [key]: value })
      saved.flash()
    } catch (err) {
      setSaveError(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  return (
    <TooltipProvider>
      <div className={cn(VIEW_WIDTH, 'flex flex-col gap-8')}>
        <div className="flex items-center justify-between gap-4">
          <h1 className="font-heading text-xl font-semibold text-foreground">Preferences</h1>
          <p role="status" className="flex items-center gap-1 text-sm text-muted-foreground">
            {saved.shown && (
              <>
                <Check className="size-4" aria-hidden="true" />
                Saved
              </>
            )}
          </p>
        </div>
        {error && (
          <p className="text-sm text-destructive">Failed to load preferences: {error.message}</p>
        )}
        {saveError && (
          <p role="alert" className="text-sm text-destructive">
            Failed to save preferences: {saveError}
          </p>
        )}
        {!error && !preferences && (
          <p className="text-sm text-muted-foreground">Loading preferences…</p>
        )}
        {preferences &&
          GROUPS.map((group) => (
            <fieldset key={group.title} className="flex flex-col gap-4">
              <legend className="mb-4 font-heading text-lg font-semibold text-foreground">
                {group.title}
              </legend>
              {group.description && (
                <p className="-mt-2 text-sm text-muted-foreground">{group.description}</p>
              )}
              {group.switches.map(({ key, label, help, needsCategories }) => (
                <div key={key} className="flex items-center justify-between gap-4">
                  <div className="flex items-center gap-1.5">
                    <Label htmlFor={`preference-${key}`}>{label}</Label>
                    <PreferenceHelp label={label} help={help} />
                  </div>
                  <Switch
                    id={`preference-${key}`}
                    checked={preferences[key]}
                    disabled={saving || (needsCategories && !preferences.categories_enabled)}
                    onCheckedChange={(value) => void toggle(key, value)}
                  />
                </div>
              ))}
            </fieldset>
          ))}
      </div>
    </TooltipProvider>
  )
}
