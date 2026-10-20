import { useState, type ReactNode } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useWatchProgress } from './useWatchProgress'
import { useCategories } from '@/api/queries'
import { aCategoryList, aVideo, mockApi } from '@/test/helpers'

afterEach(() => {
  vi.unstubAllGlobals()
  vi.useRealTimers()
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

/**
 * Plays on for the 15s the hook waits between reports while playing, so the
 * next 'timeupdate' sends one.
 */
function playUntilNextReport(element: HTMLVideoElement) {
  vi.setSystemTime(Date.now() + 15000)
  element.dispatchEvent(new Event('timeupdate'))
}

/** Lets every pending request and response handler settle. */
function settle() {
  return act(() => new Promise((resolve) => setTimeout(resolve, 0)))
}

/** A response the test sends when it chooses to. */
function deferred<T>() {
  let resolve: (value: T) => void = () => {}
  const promise = new Promise<T>((done) => {
    resolve = done
  })
  return { promise, resolve }
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

  it('reports was_watched true after a flip once playback moves on', async () => {
    const fetchMock = mockApi({ 'POST /api/videos/abc/progress': { watched: true } })
    const element = fakeVideoElement({ currentTime: 40, duration: 100, paused: false })
    const video = aVideo({ id: 'abc', watched: false })
    const { rerender } = renderHook(({ current }) => useWatchProgress(element, current), {
      wrapper: Wrapper,
      initialProps: { current: video },
    })
    rerender({ current: { ...video, watched: true } })

    element.currentTime = 5
    element.dispatchEvent(new Event('pause'))

    await waitFor(() => expect(fetchMock).toHaveBeenCalledOnce())
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({
      position_seconds: 5,
      duration_seconds: 100,
      was_watched: true,
    })
  })

  it('keeps playing when its own report marks the video watched', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    mockApi({ 'POST /api/videos/abc/progress': { watched: true } })
    const element = fakeVideoElement({ currentTime: 95, duration: 100, paused: false })
    const video = aVideo({ id: 'abc', watched: false })
    const { rerender } = renderHook(({ current }) => useWatchProgress(element, current), {
      wrapper: Wrapper,
      initialProps: { current: video },
    })
    playUntilNextReport(element)
    await settle()

    rerender({ current: { ...video, watched: true } })
    await settle()

    expect(element.paused).toBe(false)
    expect(element.currentTime).toBe(95)
  })

  it('stays at the end when finishing the video marks it watched', async () => {
    mockApi({ 'POST /api/videos/abc/progress': { watched: true } })
    const element = fakeVideoElement({ currentTime: 100, duration: 100, paused: true })
    const video = aVideo({ id: 'abc', watched: false })
    const { rerender } = renderHook(({ current }) => useWatchProgress(element, current), {
      wrapper: Wrapper,
      initialProps: { current: video },
    })
    element.dispatchEvent(new Event('ended'))
    await settle()

    rerender({ current: { ...video, watched: true } })
    await settle()

    expect(element.currentTime).toBe(100)
  })

  it('keeps playing when the list shows its own report marking the video watched before the report answers', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    const response = deferred<{ watched: boolean }>()
    mockApi({ 'POST /api/videos/abc/progress': () => response.promise })
    const element = fakeVideoElement({ currentTime: 95, duration: 100, paused: false })
    const video = aVideo({ id: 'abc', watched: false })
    const { rerender } = renderHook(({ current }) => useWatchProgress(element, current), {
      wrapper: Wrapper,
      initialProps: { current: video },
    })
    playUntilNextReport(element)

    rerender({ current: { ...video, watched: true } })
    response.resolve({ watched: true })
    await settle()

    expect(element.paused).toBe(false)
    expect(element.currentTime).toBe(95)
  })

  it('keeps playing when its own report turns a rewatched video unwatched', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    mockApi({ 'POST /api/videos/abc/progress': { watched: false } })
    const element = fakeVideoElement({ currentTime: 20, duration: 100, paused: false })
    const video = aVideo({ id: 'abc', watched: true })
    const { rerender } = renderHook(({ current }) => useWatchProgress(element, current), {
      wrapper: Wrapper,
      initialProps: { current: video },
    })
    playUntilNextReport(element)
    await settle()

    rerender({ current: { ...video, watched: false } })
    await settle()

    expect(element.paused).toBe(false)
    expect(element.currentTime).toBe(20)
  })

  it('reports was_watched true once its own report marked the video watched', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    const fetchMock = mockApi({ 'POST /api/videos/abc/progress': { watched: true } })
    const element = fakeVideoElement({ currentTime: 95, duration: 100, paused: false })
    const video = aVideo({ id: 'abc', watched: false })
    renderHook(() => useWatchProgress(element, video), { wrapper: Wrapper })
    playUntilNextReport(element)
    await settle()

    element.currentTime = 98
    element.dispatchEvent(new Event('pause'))

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2))
    const [, init] = fetchMock.mock.calls[1] as [string, RequestInit]
    expect(JSON.parse(init.body as string)).toEqual({
      position_seconds: 98,
      duration_seconds: 100,
      was_watched: true,
    })
  })

  it('still rewinds once a pending report answers unwatched while the video was marked watched', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    const response = deferred<{ watched: boolean }>()
    mockApi({ 'POST /api/videos/abc/progress': () => response.promise })
    const element = fakeVideoElement({ currentTime: 40, duration: 100, paused: false })
    const video = aVideo({ id: 'abc', watched: false })
    const { rerender } = renderHook(({ current }) => useWatchProgress(element, current), {
      wrapper: Wrapper,
      initialProps: { current: video },
    })
    playUntilNextReport(element)

    rerender({ current: { ...video, watched: true } })
    response.resolve({ watched: false })
    await settle()

    expect(element.paused).toBe(true)
    expect(element.currentTime).toBe(0)
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

  it('refetches categories when a report flips the watched state', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    const fetchMock = mockApi({
      'GET /api/categories': aCategoryList(),
      'POST /api/videos/abc/progress': { watched: true },
    })
    const element = fakeVideoElement({ currentTime: 95, duration: 100, paused: false })
    const video = aVideo({ id: 'abc', watched: false })
    renderHook(
      () => {
        useCategories()
        useWatchProgress(element, video)
      },
      { wrapper: Wrapper },
    )
    await settle()
    const categoryFetches = () =>
      fetchMock.mock.calls.filter(([url]) => url === '/api/categories').length
    const before = categoryFetches()

    playUntilNextReport(element)
    await settle()

    await waitFor(() => expect([before, categoryFetches()]).toEqual([1, 2]))
  })
})
