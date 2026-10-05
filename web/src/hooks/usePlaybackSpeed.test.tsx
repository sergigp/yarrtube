import { describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { usePlaybackSpeed, type PlaybackSpeed } from './usePlaybackSpeed'

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

  it('resets to normal speed when the player is replaced by another video', () => {
    const { result, rerender } = renderHook<
      PlaybackSpeed,
      { element: HTMLVideoElement | null; videoId: string }
    >(({ element, videoId }) => usePlaybackSpeed(element, videoId), {
      initialProps: { element: document.createElement('video'), videoId: 'abc' },
    })
    act(() => result.current.changeRate(2))

    // A video not downloaded yet has no player, and the next one gets a new one.
    rerender({ element: null, videoId: 'pending' })
    const rateWithoutPlayer = result.current.rate
    rerender({ element: document.createElement('video'), videoId: 'def' })

    expect([rateWithoutPlayer, result.current.rate]).toEqual([1, 1])
  })

  it('resets to normal speed when returning to a video on a new player', () => {
    const { result, rerender } = renderHook<
      PlaybackSpeed,
      { element: HTMLVideoElement | null; videoId: string }
    >(({ element, videoId }) => usePlaybackSpeed(element, videoId), {
      initialProps: { element: document.createElement('video'), videoId: 'abc' },
    })
    act(() => result.current.changeRate(2))

    // A video not downloaded yet has no player; going back mounts a new one,
    // already at normal speed as in browsers, so resetting it fires no
    // 'ratechange' (jsdom only reports normal speed once it is set).
    rerender({ element: null, videoId: 'pending' })
    const newPlayer = document.createElement('video')
    newPlayer.playbackRate = 1
    rerender({ element: newPlayer, videoId: 'abc' })

    expect(result.current.rate).toBe(1)
  })
})
