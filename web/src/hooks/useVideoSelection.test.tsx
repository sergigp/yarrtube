import type { ReactNode } from 'react'
import { describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { useVideoSelection } from './useVideoSelection'
import { aVideo } from '@/test/helpers'
import type { Video } from '@/api/types'

function routerAt(route: string) {
  return ({ children }: { children: ReactNode }) => (
    <MemoryRouter initialEntries={[route]}>{children}</MemoryRouter>
  )
}

describe('useVideoSelection', () => {
  it('selects nothing while the videos are still loading', () => {
    const { result } = renderHook(() => useVideoSelection(undefined), { wrapper: routerAt('/') })

    expect(result.current.selectedVideo).toBeNull()
    expect(result.current.autoplay).toBe(false)
  })

  it('defaults to the first video without autoplay', () => {
    const videos = [aVideo({ id: 'first' }), aVideo({ id: 'second' })]
    const { result } = renderHook(() => useVideoSelection(videos), { wrapper: routerAt('/') })

    expect(result.current.selectedVideo?.id).toBe('first')
    expect(result.current.autoplay).toBe(false)
  })

  it('selects a deep-linked video with autoplay', () => {
    const videos = [aVideo({ id: 'first' }), aVideo({ id: 'linked' })]
    const { result } = renderHook(() => useVideoSelection(videos), {
      wrapper: routerAt('/channels/c?video=linked'),
    })

    expect(result.current.selectedVideo?.id).toBe('linked')
    expect(result.current.autoplay).toBe(true)
  })

  it('falls back to the first video when the deep link does not match', () => {
    const videos = [aVideo({ id: 'first' })]
    const { result } = renderHook(() => useVideoSelection(videos), {
      wrapper: routerAt('/channels/c?video=missing'),
    })

    expect(result.current.selectedVideo?.id).toBe('first')
    expect(result.current.autoplay).toBe(false)
  })

  it('lets a manual pick override the deep link, without autoplay', () => {
    const videos = [aVideo({ id: 'first' }), aVideo({ id: 'linked' }), aVideo({ id: 'picked' })]
    const { result } = renderHook(() => useVideoSelection(videos), {
      wrapper: routerAt('/channels/c?video=linked'),
    })

    act(() => {
      result.current.selectVideo('picked')
    })

    expect(result.current.selectedVideo?.id).toBe('picked')
    expect(result.current.autoplay).toBe(false)
  })

  it('keeps the manual selection across a refreshed video list', () => {
    const videos = [aVideo({ id: 'first' }), aVideo({ id: 'picked' })]
    const { result, rerender } = renderHook(
      ({ list }: { list: Video[] }) => useVideoSelection(list),
      { wrapper: routerAt('/'), initialProps: { list: videos } },
    )

    act(() => {
      result.current.selectVideo('picked')
    })
    const refreshed = [
      aVideo({ id: 'brand-new' }),
      { ...videos[0]! },
      { ...videos[1]!, watched: true },
    ]
    rerender({ list: refreshed })

    expect(result.current.selectedVideo).toEqual({ ...videos[1]!, watched: true })
  })
})
