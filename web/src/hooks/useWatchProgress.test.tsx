import { useState, type ReactNode } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useWatchProgress } from './useWatchProgress'
import { aVideo, mockApi } from '@/test/helpers'

afterEach(() => {
  vi.unstubAllGlobals()
})

/**
 * A `<video>` element with the playback properties jsdom leaves
 * unimplemented made controllable.
 */
function fakeVideoElement({ currentTime = 0, duration = 300, paused = true } = {}) {
  const element = document.createElement('video')
  const state = { currentTime, duration, paused }
  Object.defineProperties(element, {
    currentTime: {
      get: () => state.currentTime,
      set: (value: number) => {
        state.currentTime = value
      },
      configurable: true,
    },
    duration: { get: () => state.duration, configurable: true },
    paused: { get: () => state.paused, configurable: true },
    readyState: { get: () => HTMLMediaElement.HAVE_METADATA, configurable: true },
  })
  element.pause = () => {
    if (!state.paused) {
      state.paused = true
      element.dispatchEvent(new Event('pause'))
    }
  }
  return element
}

function Wrapper({ children }: { children: ReactNode }) {
  // Kept across re-renders, so a rerender doesn't hand the hook a new client.
  const [queryClient] = useState(
    () => new QueryClient({ defaultOptions: { queries: { retry: false } } }),
  )
  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
}

describe('useWatchProgress', () => {
  it('resumes an unwatched video from its saved position', () => {
    const element = fakeVideoElement()
    const video = aVideo({ id: 'abc', watched: false, position_seconds: 42 })

    renderHook(() => useWatchProgress(element, video), { wrapper: Wrapper })

    expect(element.currentTime).toBe(42)
  })

  it('does not resume a watched video', () => {
    const element = fakeVideoElement()
    const video = aVideo({ id: 'abc', watched: true, position_seconds: 42 })

    renderHook(() => useWatchProgress(element, video), { wrapper: Wrapper })

    expect(element.currentTime).toBe(0)
  })

  it('reports the captured position when playback pauses', async () => {
    const fetchMock = mockApi({ 'POST /api/videos/abc/progress': { watched: false } })
    const element = fakeVideoElement({ currentTime: 63.9, duration: 300.4 })
    const video = aVideo({ id: 'abc' })
    renderHook(() => useWatchProgress(element, video), { wrapper: Wrapper })

    element.dispatchEvent(new Event('pause'))

    await waitFor(() => expect(fetchMock).toHaveBeenCalledOnce())
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({
      position_seconds: 63,
      duration_seconds: 300,
      was_watched: false,
    })
  })

  it('reports pending progress once on unmount, not twice for the same position', async () => {
    const fetchMock = mockApi({ 'POST /api/videos/abc/progress': { watched: false } })
    const element = fakeVideoElement({ currentTime: 63, duration: 300 })
    const video = aVideo({ id: 'abc' })
    const { unmount } = renderHook(() => useWatchProgress(element, video), { wrapper: Wrapper })

    element.dispatchEvent(new Event('pause'))
    await waitFor(() => expect(fetchMock).toHaveBeenCalledOnce())
    unmount()

    expect(fetchMock).toHaveBeenCalledOnce()
  })

  it('leaves an unknown duration out of the report', async () => {
    const fetchMock = mockApi({ 'POST /api/videos/abc/progress': { watched: false } })
    const element = fakeVideoElement({ currentTime: 10, duration: Number.NaN })
    const video = aVideo({ id: 'abc' })
    renderHook(() => useWatchProgress(element, video), { wrapper: Wrapper })

    element.dispatchEvent(new Event('pause'))

    await waitFor(() => expect(fetchMock).toHaveBeenCalledOnce())
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({ position_seconds: 10, was_watched: false })
  })

  it("reports was_watched as the session's state", async () => {
    const fetchMock = mockApi({ 'POST /api/videos/abc/progress': { watched: true } })
    const element = fakeVideoElement({ currentTime: 20, duration: 300 })
    const video = aVideo({ id: 'abc', watched: true })
    renderHook(() => useWatchProgress(element, video), { wrapper: Wrapper })

    element.dispatchEvent(new Event('pause'))

    await waitFor(() => expect(fetchMock).toHaveBeenCalledOnce())
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({
      position_seconds: 20,
      duration_seconds: 300,
      was_watched: true,
    })
  })

  it('pauses, rewinds and reports nothing when the video becomes watched while loaded', () => {
    const fetchMock = mockApi({})
    const element = fakeVideoElement({ currentTime: 40, duration: 100, paused: false })
    const video = aVideo({ id: 'abc', watched: false })
    const { rerender, unmount } = renderHook(({ current }) => useWatchProgress(element, current), {
      wrapper: Wrapper,
      initialProps: { current: video },
    })
    element.dispatchEvent(new Event('timeupdate'))

    rerender({ current: { ...video, watched: true } })
    unmount()

    expect(element.paused).toBe(true)
    expect(element.currentTime).toBe(0)
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('reports through a beacon when the page hides', () => {
    const sendBeacon = vi.fn(() => true)
    Object.defineProperty(window.navigator, 'sendBeacon', {
      value: sendBeacon,
      configurable: true,
      writable: true,
    })
    const element = fakeVideoElement({ currentTime: 30, duration: 300 })
    const video = aVideo({ id: 'abc' })
    renderHook(() => useWatchProgress(element, video), { wrapper: Wrapper })

    window.dispatchEvent(new Event('pagehide'))

    expect(sendBeacon).toHaveBeenCalledWith('/api/videos/abc/progress', expect.any(Blob))
  })

  it('attaches nothing while the video is not downloaded', () => {
    const fetchMock = mockApi({})
    const element = fakeVideoElement({ currentTime: 10 })
    const video = aVideo({ id: 'abc', status: 'PENDING', filename: null })
    renderHook(() => useWatchProgress(element, video), { wrapper: Wrapper })

    element.dispatchEvent(new Event('pause'))

    expect(fetchMock).not.toHaveBeenCalled()
  })
})
