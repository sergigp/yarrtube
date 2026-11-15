import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { useRememberedChoice } from './useRememberedChoice'

const CHOICES = ['a', 'b', 'c'] as const

describe('useRememberedChoice', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('starts at the initial value when nothing is remembered', () => {
    const { result } = renderHook(() => useRememberedChoice('yarrtube.test.choice', CHOICES, 'a'))

    expect(result.current[0]).toBe('a')
  })

  it('restores the remembered value, and ignores one that is no longer a choice', () => {
    window.localStorage.setItem('yarrtube.test.known', 'c')
    window.localStorage.setItem('yarrtube.test.stale', 'z')

    const known = renderHook(() => useRememberedChoice('yarrtube.test.known', CHOICES, 'a'))
    const stale = renderHook(() => useRememberedChoice('yarrtube.test.stale', CHOICES, 'a'))

    expect([known.result.current[0], stale.result.current[0]]).toEqual(['c', 'a'])
  })

  it('remembers a choice under its key', () => {
    const { result } = renderHook(() => useRememberedChoice('yarrtube.test.choice', CHOICES, 'a'))

    act(() => result.current[1]('b'))

    expect([result.current[0], window.localStorage.getItem('yarrtube.test.choice')]).toEqual([
      'b',
      'b',
    ])
  })

  it('applies a remembered value once it becomes a choice', () => {
    window.localStorage.setItem('yarrtube.test.late', 'c')
    const { result, rerender } = renderHook(
      ({ choices }) => useRememberedChoice('yarrtube.test.late', choices, 'a'),
      { initialProps: { choices: ['a'] as readonly string[] } },
    )
    const before = result.current[0]

    rerender({ choices: CHOICES })

    expect([before, result.current[0]]).toEqual(['a', 'c'])
  })

  it("reads another key's value when the key changes", () => {
    window.localStorage.setItem('yarrtube.test.two', 'c')
    const { result, rerender } = renderHook(
      ({ key }) => useRememberedChoice(key, CHOICES, 'a'),
      { initialProps: { key: 'yarrtube.test.one' } },
    )
    act(() => result.current[1]('b'))

    rerender({ key: 'yarrtube.test.two' })

    expect(result.current[0]).toBe('c')
  })

  it('falls back to the initial value when storage throws, and still changes', () => {
    const blocked = () => {
      throw new DOMException('blocked', 'SecurityError')
    }
    vi.spyOn(window.localStorage, 'getItem').mockImplementation(blocked)
    vi.spyOn(window.localStorage, 'setItem').mockImplementation(blocked)
    const { result } = renderHook(() => useRememberedChoice('yarrtube.test.choice', CHOICES, 'a'))
    const initial = result.current[0]

    act(() => result.current[1]('c'))

    expect([initial, result.current[0]]).toEqual(['a', 'c'])
  })
})
