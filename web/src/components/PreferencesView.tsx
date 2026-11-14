import { useState } from 'react'
import { usePreferences, useSavePreferences } from '@/api/queries'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { PageHeader } from '@/components/PageHeader'
import { SettingsCard } from '@/components/SettingsCard'
import { cn } from '@/lib/utils'
import { VIEW_WIDTH } from '@/lib/viewWidth'
import { errorMessage } from '@/lib/errorMessage'
import { useSavedFlash } from '@/hooks/useSavedFlash'

/** The preferences shown as switches here; the theme has its header toggle. */
type PreferenceKey =
  | 'categories_enabled'
  | 'home_show_continue_watching'
  | 'home_show_categories'
  | 'home_show_quick_watches'
  | 'home_show_latest'

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
        label: 'Categories',
        help: 'Group channels and playlists by topic and browse by category. Turning this off hides categories but keeps them and their videos.',
      },
    ],
  },
  {
    title: 'Home sections',
    description:
      'Shown top to bottom in this order. A video appears in one section only, so turning a section off can move its videos to the next one.',
    switches: [
      {
        key: 'home_show_continue_watching',
        label: 'Continue watching',
        help: "Videos you started in the last 7 days and haven't finished.",
      },
      {
        key: 'home_show_categories',
        label: 'More about…',
        help: 'One section for each of up to 3 categories with unwatched videos.',
        needsCategories: true,
      },
      {
        key: 'home_show_quick_watches',
        label: 'Quick watches',
        help: 'Unwatched videos under 15 minutes.',
      },
      {
        key: 'home_show_latest',
        label: 'Latest videos',
        help: "Your newest downloads that aren't in a section above.",
      },
    ],
  },
]

/** The preferences view: a switch per preference, saved as soon as it is toggled. */
export function PreferencesView() {
  const { data: preferences, error } = usePreferences()
  const savePreferences = useSavePreferences()
  const [saveError, setSaveError] = useState<string | null>(null)
  const saved = useSavedFlash()
  // One save at a time, so the saved values shown never race each other.
  const [saving, setSaving] = useState(false)
  const toggle = async (key: PreferenceKey, value: boolean) => {
    if (!preferences || saving) {
      return
    }
    setSaving(true)
    setSaveError(null)
    saved.clear()
    try {
      await savePreferences({ [key]: value })
      saved.flash()
    } catch (err) {
      setSaveError(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className={cn(VIEW_WIDTH, 'flex flex-col gap-8')}>
      <PageHeader
        title="General"
        description="Changes save as you make them."
        saved={saved.shown}
      />
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
          <SettingsCard
            key={group.title}
            title={group.title}
            description={group.description}
          >
            <div className="flex flex-col divide-y divide-border">
            {group.switches.map(({ key, label, help, needsCategories }) => (
              <div
                key={key}
                className="flex items-center justify-between gap-6 py-3.5 first:pt-0 last:pb-0"
              >
                <div className="flex flex-col gap-0.5">
                  <Label htmlFor={`preference-${key}`} className="text-[15px]">
                    {label}
                  </Label>
                  <p id={`preference-${key}-help`} className="text-sm text-muted-foreground">
                    {help}
                  </p>
                </div>
                <Switch
                  id={`preference-${key}`}
                  aria-describedby={`preference-${key}-help`}
                  checked={preferences[key]}
                  disabled={saving || (needsCategories && !preferences.categories_enabled)}
                  onCheckedChange={(value) => void toggle(key, value)}
                />
              </div>
            ))}
            </div>
          </SettingsCard>
        ))}
    </div>
  )
}
