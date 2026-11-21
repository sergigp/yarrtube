import { beforeEach } from 'vitest'
import { PHONE_LANDSCAPE_QUERY } from '@/hooks/useIsPhoneLandscape'

/** The viewport the `matchMedia` stub in `setup.ts` reports; a portrait phone by default. */
export const viewport = { desktop: false, phoneLandscape: false }

/** The `change` listeners registered on the `matchMedia` stub, per query. */
export const mediaListeners = new Map<string, Set<() => void>>()

/** Whether the `matchMedia` stub matches `query` on the current viewport. */
export function matchesQuery(query: string): boolean {
  return query === PHONE_LANDSCAPE_QUERY ? viewport.phoneLandscape : viewport.desktop
}

/** Runs the calling `describe`'s tests on a desktop viewport. */
export function useDesktopViewport(): void {
  beforeEach(() => {
    viewport.desktop = true
  })
}

/** Turns the emulated phone and notifies `matchMedia` change listeners. */
export function rotatePhone(orientation: 'landscape' | 'portrait'): void {
  viewport.phoneLandscape = orientation === 'landscape'
  for (const listeners of mediaListeners.values()) {
    for (const listener of [...listeners]) listener()
  }
}
