import { describe, expect, it } from 'vitest'
import { seekDirection, seekTime } from './seekKeys'

function keydown(key: string, init: KeyboardEventInit = {}, target: Element = document.body) {
  const event = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init })
  target.dispatchEvent(event)
  return event
}

describe('seekDirection', () => {
  it('reads ← as back and → as forward', () => {
    expect(seekDirection(keydown('ArrowLeft'))).toBe('back')
    expect(seekDirection(keydown('ArrowRight'))).toBe('forward')
  })

  it('ignores other keys and modified arrows', () => {
    expect(seekDirection(keydown('ArrowUp'))).toBeNull()
    expect(seekDirection(keydown('ArrowRight', { shiftKey: true }))).toBeNull()
    expect(seekDirection(keydown('ArrowRight', { metaKey: true }))).toBeNull()
  })

  it('ignores arrows typed into a field', () => {
    const input = document.createElement('input')
    document.body.append(input)

    expect(seekDirection(keydown('ArrowLeft', {}, input))).toBeNull()
    input.remove()
  })

  it('ignores arrows something else already handled', () => {
    const event = new KeyboardEvent('keydown', { key: 'ArrowLeft', cancelable: true })
    event.preventDefault()

    expect(seekDirection(event)).toBeNull()
  })
})

describe('seekTime', () => {
  it('moves ten seconds either way', () => {
    expect(seekTime(30, 100, 'forward')).toBe(40)
    expect(seekTime(30, 100, 'back')).toBe(20)
  })

  it('stays within the video', () => {
    expect(seekTime(4, 100, 'back')).toBe(0)
    expect(seekTime(95, 100, 'forward')).toBe(100)
  })

  it('seeks forward freely while the duration is unknown', () => {
    expect(seekTime(30, Number.NaN, 'forward')).toBe(40)
  })
})
