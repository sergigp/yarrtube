import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { useRememberedFlag } from './useRememberedFlag'

describe('useRememberedFlag', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('starts at the initial value when nothing is remembered', () => {
    const { result } = renderHook(() => useRememberedFlag('yarrtube.test.flag', true))

    expect(result.current[0]).toBe(true)
  })

  it('restores the remembered value', () => {
    window.localStorage.setItem('yarrtube.test.flag', 'true')

    const { result } = renderHook(() => useRememberedFlag('yarrtube.test.flag', false))

    expect(result.current[0]).toBe(true)
  })

  it('remembers a toggle under its key', () => {
    const { result } = renderHook(() => useRememberedFlag('yarrtube.test.flag', false))

    act(() => result.current[1]())

    expect([result.current[0], window.localStorage.getItem('yarrtube.test.flag')]).toEqual([
      true,
      'true',
    ])
  })
})
