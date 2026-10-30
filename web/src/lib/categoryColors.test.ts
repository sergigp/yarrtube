import { describe, expect, it } from 'vitest'
import { PALETTE, defaultColor, hueOf, inkShade, softTint, solidFromHue } from './categoryColors'

const channels = (hex: string) => [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16))
const hueDistance = (a: number, b: number) => Math.min(Math.abs(a - b), 360 - Math.abs(a - b))

/** WCAG contrast ratio between two `#rrggbb` colors. */
function contrast(a: string, b: string): number {
  const luminance = (hex: string) => {
    const [r, g, bl] = channels(hex).map((byte) => {
      const c = byte / 255
      return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
    })
    return 0.2126 * r + 0.7152 * g + 0.0722 * bl
  }
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x)
  return (light + 0.05) / (dark + 0.05)
}

describe('categoryColors', () => {
  it('names the solid palette in the order defaults are handed out', () => {
    expect(PALETTE).toEqual([
      { name: 'Mustard', hex: '#9f8401' },
      { name: 'Orange', hex: '#bd7123' },
      { name: 'Coral', hex: '#c8645b' },
      { name: 'Rose', hex: '#bd6396' },
      { name: 'Violet', hex: '#9870c5' },
      { name: 'Periwinkle', hex: '#6a80d4' },
      { name: 'Sky', hex: '#0193bf' },
      { name: 'Teal', hex: '#019b8d' },
      { name: 'Olive', hex: '#79922c' },
      { name: 'Taupe', hex: '#978364' },
    ])
  })

  it('defaults to the least used palette color, the earliest on a tie', () => {
    const everyColorAndMustardAgain = [...PALETTE.map((color) => color.hex), '#9f8401']

    expect([
      defaultColor([]),
      defaultColor(['#9f8401', '#c8645b']),
      defaultColor(everyColorAndMustardAgain),
      defaultColor(['#f0e4d0']),
    ]).toEqual(['#9f8401', '#bd7123', '#bd7123', '#9f8401'])
  })

  it('makes the solid color of every hue, whose hue reads back within 3 degrees', () => {
    const hues = Array.from({ length: 24 }, (_, index) => index * 15)

    const solids = hues.map(solidFromHue)

    expect(solids.filter((hex) => !/^#[0-9a-f]{6}$/.test(hex))).toEqual([])
    expect(hues.filter((hue, index) => hueDistance(hueOf(solids[index]), hue) > 3)).toEqual([])
    expect(solidFromHue(hueOf('#d3ecf8'))).toBe('#0193bf')
  })

  it("derives a soft tint of a color's hue, light enough for a background", () => {
    expect(PALETTE.filter((color) => Math.min(...channels(softTint(color.hex))) < 200)).toEqual([])
    expect(
      PALETTE.filter((color) => hueDistance(hueOf(softTint(color.hex)), hueOf(color.hex)) > 3),
    ).toEqual([])
  })

  it('derives an ink shade readable on the soft tint, at least 4.5:1', () => {
    expect(
      PALETTE.filter((color) => contrast(inkShade(color.hex), softTint(color.hex)) < 4.5),
    ).toEqual([])
  })
})
