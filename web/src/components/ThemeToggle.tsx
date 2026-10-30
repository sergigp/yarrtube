import { Moon, Sun } from 'lucide-react'
import { Switch as SwitchPrimitive } from 'radix-ui'
import { usePreferences, useSaveTheme } from '@/api/queries'
import { applyTheme, useTheme } from '@/hooks/useTheme'

/**
 * The header's light/dark switch: a round knob that slides right into dark
 * mode, showing a sun in light mode and a moon in dark mode. The theme is
 * drawn at once and saved as a preference; a failed save draws the saved
 * one back. Disabled until the preferences load, since saving sends them all.
 */
export function ThemeToggle() {
  const theme = useTheme()
  const { data: preferences } = usePreferences()
  const saveTheme = useSaveTheme()
  const dark = theme === 'dark'

  const toggle = async (checked: boolean) => {
    if (!preferences) {
      return
    }
    applyTheme(checked ? 'dark' : 'light')
    try {
      await saveTheme(preferences, checked ? 'dark' : 'light')
    } catch {
      applyTheme(preferences.theme)
    }
  }

  return (
    <SwitchPrimitive.Root
      aria-label="Dark mode"
      checked={dark}
      disabled={!preferences}
      onCheckedChange={toggle}
      className="inline-flex h-7 w-12 shrink-0 items-center rounded-full px-0.5 transition-colors outline-none focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50 data-[state=checked]:bg-primary data-[state=unchecked]:bg-input"
    >
      <SwitchPrimitive.Thumb className="pointer-events-none flex size-6 items-center justify-center rounded-full bg-white shadow-sm transition-transform duration-200 data-[state=checked]:translate-x-5 data-[state=unchecked]:translate-x-0">
        {dark ? (
          <Moon aria-hidden="true" className="size-3.5 fill-current text-primary" />
        ) : (
          <Sun aria-hidden="true" className="size-3.5 text-amber-500" />
        )}
      </SwitchPrimitive.Thumb>
    </SwitchPrimitive.Root>
  )
}
