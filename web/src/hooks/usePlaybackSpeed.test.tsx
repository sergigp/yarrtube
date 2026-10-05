import { describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { usePlaybackSpeed } from './usePlaybackSpeed'

describe('usePlaybackSpeed', () => {
  it('starts at normal speed', () => {
    const element = document.createElement('video')

    const { result } = renderHook(() => usePlaybackSpeed(element, 'abc'))

    expect(result.current.rate).toBe(1)
  })

  it('applies a chosen speed to the video element', () => {
    const element = document.createElement('video')
    const { result } = renderHook(() => usePlaybackSpeed(element, 'abc'))

    act(() => result.current.changeRate(1.5))

    expect(element.playbackRate).toBe(1.5)
    expect(result.current.rate).toBe(1.5)
  })

  it('follows speed changes made by the native controls', () => {
    const element = document.createElement('video')
    const { result } = renderHook(() => usePlaybackSpeed(element, 'abc'))

    // jsdom fires 'ratechange' when the speed is set, as browsers do.
    act(() => {
      element.playbackRate = 1.75
    })

    expect(result.current.rate).toBe(1.75)
  })

  it('resets to normal speed when another video is selected', () => {
    const element = document.createElement('video')
    const { result, rerender } = renderHook(
      ({ videoId }) => usePlaybackSpeed(element, videoId),
      { initialProps: { videoId: 'abc' } },
    )
    act(() => result.current.changeRate(2))

    rerender({ videoId: 'def' })

    expect(element.playbackRate).toBe(1)
    expect(result.current.rate).toBe(1)
  })
})
