import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { useDebouncedValue } from './useDebouncedValue'

describe('useDebouncedValue', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('starts with the initial value', () => {
    const { result } = renderHook(() => useDebouncedValue('initial', 400))

    expect(result.current).toBe('initial')
  })

  it('keeps the old value until the delay has passed', () => {
    const { result, rerender } = renderHook(({ value }) => useDebouncedValue(value, 400), {
      initialProps: { value: 'first' },
    })

    rerender({ value: 'second' })
    act(() => {
      vi.advanceTimersByTime(399)
    })

    expect(result.current).toBe('first')

    act(() => {
      vi.advanceTimersByTime(1)
    })

    expect(result.current).toBe('second')
  })

  it('restarts the delay on every change', () => {
    const { result, rerender } = renderHook(({ value }) => useDebouncedValue(value, 400), {
      initialProps: { value: 'first' },
    })

    rerender({ value: 'second' })
    act(() => {
      vi.advanceTimersByTime(300)
    })
    rerender({ value: 'third' })
    act(() => {
      vi.advanceTimersByTime(300)
    })

    expect(result.current).toBe('first')

    act(() => {
      vi.advanceTimersByTime(100)
    })

    expect(result.current).toBe('third')
  })
})
