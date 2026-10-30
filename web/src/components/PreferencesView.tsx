import { usePreferences, useSavePreferences } from '@/api/queries'
import type { Preferences } from '@/api/types'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'

type PreferenceKey = keyof Preferences

interface PreferenceGroup {
  title: string
  switches: { key: PreferenceKey; label: string }[]
}

const GROUPS: PreferenceGroup[] = [
  {
    title: 'Categories',
    switches: [{ key: 'categories_enabled', label: 'Enable categories' }],
  },
  {
    title: 'Home',
    switches: [
      { key: 'home_show_continue_watching', label: 'Show continue watching' },
      { key: 'home_show_categories', label: 'Show categories' },
      { key: 'home_show_category_colors', label: 'Show category colors' },
      { key: 'home_show_quick_watches', label: 'Show quick watches' },
      { key: 'home_show_latest', label: 'Show latest videos' },
    ],
  },
]

/** The preferences view: a switch per preference, saved as soon as it is toggled. */
export function PreferencesView() {
  const { data: preferences, error } = usePreferences()
  const savePreferences = useSavePreferences()
  const toggle = (key: PreferenceKey, value: boolean) => {
    if (preferences) {
      void savePreferences({ ...preferences, [key]: value })
    }
  }

  return (
    <div className="flex max-w-xl flex-col gap-8">
      <h1 className="font-heading text-xl font-semibold text-foreground">Preferences</h1>
      {error && (
        <p className="text-sm text-destructive">Failed to load preferences: {error.message}</p>
      )}
      {!error && !preferences && <p className="text-sm text-muted-foreground">Loading preferences…</p>}
      {preferences &&
        GROUPS.map((group) => (
          <fieldset key={group.title} className="flex flex-col gap-4">
            <legend className="mb-4 font-heading text-lg font-semibold text-foreground">
              {group.title}
            </legend>
            {group.switches.map(({ key, label }) => (
              <div key={key} className="flex items-center justify-between gap-4">
                <Label htmlFor={`preference-${key}`}>{label}</Label>
                <Switch
                  id={`preference-${key}`}
                  checked={preferences[key]}
                  onCheckedChange={(value) => toggle(key, value)}
                />
              </div>
            ))}
          </fieldset>
        ))}
    </div>
  )
}
