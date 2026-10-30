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
      { name: 'Amber', hex: '#f59e0b' },
      { name: 'Orange', hex: '#f97316' },
      { name: 'Red', hex: '#ef4444' },
      { name: 'Pink', hex: '#ec4899' },
      { name: 'Violet', hex: '#8b5cf6' },
      { name: 'Blue', hex: '#3b82f6' },
      { name: 'Sky', hex: '#0ea5e9' },
      { name: 'Teal', hex: '#14b8a6' },
      { name: 'Green', hex: '#22c55e' },
      { name: 'Slate', hex: '#64748b' },
    ])
  })

  it('defaults to the least used palette color, the earliest on a tie', () => {
    const everyColorAndAmberAgain = [...PALETTE.map((color) => color.hex), '#f59e0b']

    expect([
      defaultColor([]),
      defaultColor(['#f59e0b', '#ef4444']),
      defaultColor(everyColorAndAmberAgain),
      defaultColor(['#f0e4d0']),
    ]).toEqual(['#f59e0b', '#f97316', '#f97316', '#f59e0b'])
  })

  it('makes the custom color of every hue, whose hue reads back within 3 degrees', () => {
    const hues = Array.from({ length: 24 }, (_, index) => index * 15)

    const solids = hues.map(solidFromHue)

    expect(solids.filter((hex) => !/^#[0-9a-f]{6}$/.test(hex))).toEqual([])
    expect(hues.filter((hue, index) => hueDistance(hueOf(solids[index]), hue) > 3)).toEqual([])
    expect(solidFromHue(hueOf('#0ea5e9'))).toBe('#039ada')
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
