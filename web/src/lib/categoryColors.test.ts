import { describe, expect, it } from 'vitest'
import { cardBackground, defaultColor, hueOf, pastelFromHue } from './categoryColors'

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

  it('tints a card flat for one category, in diagonal bands for 2–3 and grey for 4 or more', () => {
    expect([
      cardBackground([]),
      cardBackground(['#d3ecf8']),
      cardBackground(['#d3ecf8', '#fff1bf']),
      cardBackground(['#d3ecf8', '#fff1bf', '#e9def8']),
      cardBackground(['#d3ecf8', '#fff1bf', '#e9def8', '#ffd9d4']),
      cardBackground(['#d3ecf8', '#fff1bf', '#e9def8', '#ffd9d4', '#cff0ea']),
    ]).toEqual([
      undefined,
      '#d3ecf8',
      'linear-gradient(to bottom right, #d3ecf8 0% 50%, #fff1bf 50% 100%)',
      'linear-gradient(to bottom right, #d3ecf8 0% 33.33%, #fff1bf 33.33% 66.67%, #e9def8 66.67% 100%)',
      '#f2f2f2',
      '#f2f2f2',
    ])
  })

  it('makes a soft pastel of every hue, whose hue reads back within 3 degrees', () => {
    const hues = Array.from({ length: 24 }, (_, index) => index * 15)
    const channels = (hex: string) => [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16))
    const hueDistance = (a: number, b: number) => Math.min(Math.abs(a - b), 360 - Math.abs(a - b))

    const pastels = hues.map(pastelFromHue)

    expect(pastels.filter((hex) => !/^#[0-9a-f]{6}$/.test(hex))).toEqual([])
    expect(pastels.filter((hex) => Math.min(...channels(hex)) < 200)).toEqual([])
    expect(hues.filter((hue, index) => hueDistance(hueOf(pastels[index]), hue) > 3)).toEqual([])
  })
})
