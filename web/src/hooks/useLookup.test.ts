import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import type { UseQueryResult } from '@tanstack/react-query'
import { useLookup } from './useLookup'

interface Preview {
  id: string
  title: string
}

/** A controllable stand-in for a preview query hook. */
function previewStub(state: {
  data?: Preview
  error?: Error
  isFetching?: boolean
}): (value: string) => UseQueryResult<Preview, Error> {
  return () =>
    ({
      data: state.data,
      error: state.error ?? null,
      isFetching: state.isFetching ?? false,
    }) as UseQueryResult<Preview, Error>
}

const sameId = (a: string, b: string) => a === b

describe('useLookup', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('is looking up until the debounce catches up with the entered value', () => {
    const usePreview = previewStub({ data: { id: 'x', title: 'X' } })
    const { result, rerender } = renderHook(
      ({ entered }) => useLookup(entered, usePreview, [], sameId),
      { initialProps: { entered: 'a' } },
    )

    rerender({ entered: 'ab' })

    expect(result.current.lookingUp).toBe(true)
    expect(result.current.addable).toBe(false)

    act(() => {
      vi.advanceTimersByTime(400)
    })

    expect(result.current.lookingUp).toBe(false)
  })

  it('is looking up while the preview query is fetching', () => {
    const { result } = renderHook(() =>
      useLookup('a', previewStub({ isFetching: true }), [], sameId),
    )

    expect(result.current.lookingUp).toBe(true)
    expect(result.current.addable).toBe(false)
  })

  it('is addable once something untracked is found and the lookup is current', () => {
    const usePreview = previewStub({ data: { id: 'new', title: 'New' } })
    const { result } = renderHook(() =>
      useLookup('new', usePreview, [{ id: 'other', name: 'Other' }], sameId),
    )

    expect(result.current.addable).toBe(true)
    expect(result.current.tracked).toBeUndefined()
  })

  it('reports the tracked item instead of addable when it is already added', () => {
    const usePreview = previewStub({ data: { id: 'dup', title: 'Dup' } })
    const { result } = renderHook(() =>
      useLookup('dup', usePreview, [{ id: 'dup', name: 'Existing' }], sameId),
    )

    expect(result.current.addable).toBe(false)
    expect(result.current.tracked).toEqual({ id: 'dup', name: 'Existing' })
  })

  it('matches tracked items through the sameId comparator', () => {
    const usePreview = previewStub({ data: { id: 'MiXeD', title: 'Mixed' } })
    const caseInsensitive = (a: string, b: string) => a.toLowerCase() === b.toLowerCase()
    const { result } = renderHook(() =>
      useLookup('mixed', usePreview, [{ id: 'mixed', name: 'Existing' }], caseInsensitive),
    )

    expect(result.current.tracked).toEqual({ id: 'mixed', name: 'Existing' })
  })

  it('is not addable while nothing has been found', () => {
    const { result } = renderHook(() => useLookup('a', previewStub({}), [], sameId))

    expect(result.current.addable).toBe(false)
  })
})
