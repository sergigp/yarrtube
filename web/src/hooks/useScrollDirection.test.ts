import { afterEach, describe, expect, it } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { useScrollDirection } from './useScrollDirection'

function scrollTo(y: number) {
  act(() => {
    Object.defineProperty(window, 'scrollY', { value: y, configurable: true })
    window.dispatchEvent(new Event('scroll'))
  })
}

describe('useScrollDirection', () => {
  afterEach(() => {
    scrollTo(0)
  })

  it('reads down while the document scrolls down and up as soon as it scrolls back up', () => {
    const { result } = renderHook(() => useScrollDirection())

    const directions = [result.current]
    scrollTo(100)
    directions.push(result.current)
    scrollTo(300)
    directions.push(result.current)
    scrollTo(290)
    directions.push(result.current)

    expect(directions).toEqual(['up', 'down', 'down', 'up'])
  })

  it('ignores movements under 8 pixels, such as rubber-banding', () => {
    const { result } = renderHook(() => useScrollDirection())

    scrollTo(100)
    scrollTo(95)

    expect(result.current).toBe('down')
  })

  it('reads up at the top of the document, also when it overscrolls above it', () => {
    const { result } = renderHook(() => useScrollDirection())

    scrollTo(100)
    scrollTo(-20)
    const overscrolled = result.current
    scrollTo(100)
    scrollTo(0)

    expect([overscrolled, result.current]).toEqual(['up', 'up'])
  })
})
