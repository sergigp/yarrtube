import { beforeEach } from 'vitest'

/** The viewport the `matchMedia` stub in `setup.ts` reports; mobile by default. */
export const viewport = { desktop: false }

/** Runs the calling `describe`'s tests on a desktop viewport. */
export function useDesktopViewport(): void {
  beforeEach(() => {
    viewport.desktop = true
  })
}
