import '@testing-library/jest-dom/vitest'
import { afterEach } from 'vitest'
import { listenersFor, matchesQuery, mediaListeners, viewport } from './viewport'

// jsdom implements neither `matchMedia` nor `ResizeObserver`; the components
// only need them to exist (VideoDetail starts expanded on desktop widths,
// Description measures overflow). Queries match per `matchesQuery`: the
// phone-landscape query on `viewport.phoneLandscape` (toggled by
// `rotatePhone()`, which fires their `change` listeners), every other query
// only on a desktop viewport, which `useDesktopViewport()` opts a test into.
afterEach(() => {
  viewport.desktop = false
  viewport.phoneLandscape = false
  mediaListeners.clear()
  // The theme lives on <html>, which outlives each test's render.
  document.documentElement.classList.remove('dark')
})

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => {
    return {
      get matches() {
        return matchesQuery(query)
      },
      media: query,
      onchange: null,
      addEventListener: (_type: string, listener: () => void) => listenersFor(query).add(listener),
      removeEventListener: (_type: string, listener: () => void) => listenersFor(query).delete(listener),
      addListener: (listener: () => void) => listenersFor(query).add(listener),
      removeListener: (listener: () => void) => listenersFor(query).delete(listener),
      dispatchEvent: () => false,
    }
  },
})

class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}

if (!('ResizeObserver' in globalThis)) {
  Object.defineProperty(globalThis, 'ResizeObserver', {
    writable: true,
    value: ResizeObserverStub,
  })
}

// Node's own experimental `localStorage` (a limited stub without `clear`)
// shadows jsdom's; give tests a real in-memory Storage instead.
class MemoryStorage {
  private store = new Map<string, string>()

  getItem(key: string): string | null {
    return this.store.get(key) ?? null
  }

  setItem(key: string, value: string): void {
    this.store.set(key, String(value))
  }

  removeItem(key: string): void {
    this.store.delete(key)
  }

  clear(): void {
    this.store.clear()
  }

  key(index: number): string | null {
    return [...this.store.keys()][index] ?? null
  }

  get length(): number {
    return this.store.size
  }
}

Object.defineProperty(window, 'localStorage', {
  value: new MemoryStorage(),
  configurable: true,
  writable: true,
})

// Radix UI's pointer handling probes APIs jsdom lacks.
if (!Element.prototype.hasPointerCapture) {
  Element.prototype.hasPointerCapture = () => false
  Element.prototype.setPointerCapture = () => {}
  Element.prototype.releasePointerCapture = () => {}
}
if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {}
}
// jsdom has no layout, so no element scrolling either (the chapter rail scrolls itself).
if (!Element.prototype.scrollTo) {
  Element.prototype.scrollTo = () => {}
}
// jsdom has no `window.scrollTo` either (it logs "Not implemented").
window.scrollTo = () => {}

// jsdom has no media playback: its `play()`/`pause()`/`load()` log "Not implemented".
HTMLMediaElement.prototype.play = () => Promise.resolve()
HTMLMediaElement.prototype.pause = () => {}
HTMLMediaElement.prototype.load = () => {}
