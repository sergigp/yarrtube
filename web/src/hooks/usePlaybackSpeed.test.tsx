import { describe, expect, it } from 'vitest'
import { renderHook } from '@testing-library/react'
import { usePlaybackSpeed } from './usePlaybackSpeed'

describe('usePlaybackSpeed', () => {
  it('starts at normal speed', () => {
    const element = document.createElement('video')

    const { result } = renderHook(() => usePlaybackSpeed(element, 'abc'))

    expect(result.current.rate).toBe(1)
  })
})
