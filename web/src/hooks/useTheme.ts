import { useEffect, useSyncExternalStore } from 'react'
import { usePreferences } from '@/api/queries'
import { THEME_KEY, type Theme } from '@/lib/theme'

/** The browser chrome color per theme, matching `--background`. */
const THEME_COLORS: Record<Theme, string> = { light: '#ffffff', dark: '#0f0f0f' }

const listeners = new Set<() => void>()

/**
 * The theme the UI is drawn in, kept current. The `dark` class on `<html>`
 * is what's drawn: `index.html` sets it from the cache before first paint.
 */
export function useTheme(): Theme {
  return useSyncExternalStore(subscribe, currentTheme)
}

/** Draws the UI in the saved theme once preferences load, and whenever it changes. */
export function useSavedTheme(): void {
  const { data } = usePreferences()
  const saved = data?.theme
  useEffect(() => {
    if (saved) {
      applyTheme(saved)
    }
  }, [saved])
}

/**
 * Draws the UI in `theme` and caches it for the next first paint.
 * Storage unwritable: only the next load's first paint may differ.
 */
export function applyTheme(theme: Theme): void {
  document.documentElement.classList.toggle('dark', theme === 'dark')
  document.querySelector('meta[name="theme-color"]')?.setAttribute('content', THEME_COLORS[theme])
  try {
    window.localStorage.setItem(THEME_KEY, theme)
  } catch {
    // Not cached; the saved preference still applies once loaded.
  }
  listeners.forEach((listener) => listener())
}

function currentTheme(): Theme {
  return document.documentElement.classList.contains('dark') ? 'dark' : 'light'
}

function subscribe(onChange: () => void): () => void {
  listeners.add(onChange)
  return () => listeners.delete(onChange)
}
