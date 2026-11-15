import { Moon, Sun } from 'lucide-react'
import { usePreferences, useSaveTheme } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { applyTheme, useTheme } from '@/hooks/useTheme'

/**
 * The header's light/dark control: a plain icon button showing the current
 * theme (a moon in dark mode, a sun in light) and naming the one it switches
 * to. The theme is drawn at once and saved as a preference; a failed save
 * draws the saved one back. Disabled until the preferences load, the theme to
 * fall back to.
 */
export function ThemeToggle() {
  const theme = useTheme()
  const { data: preferences } = usePreferences()
  const saveTheme = useSaveTheme()
  const dark = theme === 'dark'
  const label = dark ? 'Switch to light theme' : 'Switch to dark theme'

  const toggle = async () => {
    if (!preferences) {
      return
    }
    const next = dark ? 'light' : 'dark'
    applyTheme(next)
    try {
      await saveTheme(next)
    } catch {
      applyTheme(preferences.theme)
    }
  }

  return (
    <Button
      variant="ghost"
      size="icon"
      aria-label={label}
      title={label}
      disabled={!preferences}
      onClick={toggle}
    >
      {dark ? <Moon aria-hidden="true" className="size-5" /> : <Sun aria-hidden="true" className="size-5" />}
    </Button>
  )
}
