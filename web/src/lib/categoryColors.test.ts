import { describe, expect, it } from 'vitest'
import { defaultColor } from './categoryColors'

describe('categoryColors', () => {
  it('defaults to the least used palette color, the earliest on a tie', () => {
    const everyColorAndButterAgain = [
      '#fff1bf',
      '#ffe0c7',
      '#ffd9d4',
      '#fadceb',
      '#e9def8',
      '#dbe3fb',
      '#d3ecf8',
      '#cff0ea',
      '#e4f0cc',
      '#eee5d7',
      '#fff1bf',
    ]

    expect([
      defaultColor([]),
      defaultColor(['#fff1bf', '#ffd9d4']),
      defaultColor(everyColorAndButterAgain),
      defaultColor(['#f0e4d0']),
    ]).toEqual(['#fff1bf', '#ffe0c7', '#ffe0c7', '#fff1bf'])
  })
})
