/** The category color presets, in the order defaults are handed out. Mirrors the daemon's palette. */
export const PALETTE: readonly { name: string; hex: string }[] = [
  { name: 'Butter', hex: '#fff1bf' },
  { name: 'Peach', hex: '#ffe0c7' },
  { name: 'Coral', hex: '#ffd9d4' },
  { name: 'Rose', hex: '#fadceb' },
  { name: 'Lilac', hex: '#e9def8' },
  { name: 'Periwinkle', hex: '#dbe3fb' },
  { name: 'Sky', hex: '#d3ecf8' },
  { name: 'Aqua', hex: '#cff0ea' },
  { name: 'Pistachio', hex: '#e4f0cc' },
  { name: 'Sand', hex: '#eee5d7' },
]

/** Background of a card in 4 or more categories. */
export const CROWDED_CARD = '#f2f2f2'

/** The palette color used by the fewest of `used`, the earliest on a tie. */
export function defaultColor(_used: string[]): string {
  return PALETTE[0].hex
}

/** The soft pastel of `hue` (degrees): oklch(0.94 0.04 hue) as `#rrggbb`. */
export function pastelFromHue(_hue: number): string {
  return PALETTE[0].hex
}

/** The oklch hue of `hex`, in degrees. */
export function hueOf(_hex: string): number {
  return 0
}

/**
 * A card's background for its categories' colors: none for 0, flat for 1,
 * diagonal hard-stop bands for 2–3, `CROWDED_CARD` for 4 or more.
 */
export function cardBackground(_colors: string[]): string | undefined {
  return undefined
}
