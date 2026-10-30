/** The category color presets, in the order defaults are handed out. Mirrors the daemon's palette. */
export const PALETTE: readonly { name: string; hex: string }[] = [
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
]

/** The palette color used by the fewest of `used`, the earliest on a tie. */
export function defaultColor(used: string[]): string {
  const uses = (hex: string) => used.filter((color) => color === hex).length
  return PALETTE.reduce((least, color) => (uses(color.hex) < uses(least.hex) ? color : least)).hex
}

/** The palette's solid lightness and chroma: what a category stores. */
const SOLID = { lightness: 0.62, chroma: 0.13 }
/** Chip and category header backgrounds. */
const SOFT = { lightness: 0.94, chroma: 0.04 }
/** Chip text, readable on the soft tint. */
const INK = { lightness: 0.42, chroma: 0.1 }

/** The solid color of `hue` (degrees), as the custom hue slider picks it. */
export function solidFromHue(hue: number): string {
  return fromOklch(SOLID.lightness, SOLID.chroma, hue)
}

/** The soft tint of `hex`'s hue: chip and category header backgrounds. */
export function softTint(hex: string): string {
  return fromOklch(SOFT.lightness, SOFT.chroma, hueOf(hex))
}

/** The dark shade of `hex`'s hue: chip text, readable on `softTint(hex)`. */
export function inkShade(hex: string): string {
  return fromOklch(INK.lightness, INK.chroma, hueOf(hex))
}

/** The oklch hue of `hex`, in degrees, from 0 up to 360. */
export function hueOf(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((at) => gammaDecode(parseInt(hex.slice(at, at + 2), 16) / 255))
  const l = Math.cbrt(0.4122214708 * r + 0.5363329363 * g + 0.0514459929 * b)
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
  const labA = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s
  const labB = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s
  return ((Math.atan2(labB, labA) * 180) / Math.PI + 360) % 360
}

/**
 * oklch(lightness chroma hue) as `#rrggbb`. Where that falls outside sRGB, the
 * chroma drops until it fits, so the lightness and hue hold.
 */
function fromOklch(lightness: number, startChroma: number, hue: number): string {
  let chroma = startChroma
  let rgb = oklchToLinearRgb(lightness, chroma, hue)
  while (rgb.some((channel) => channel < 0 || channel > 1)) {
    chroma -= 0.001
    rgb = oklchToLinearRgb(lightness, chroma, hue)
  }
  return `#${rgb.map((channel) => toHexByte(gammaEncode(channel))).join('')}`
}

/** OKLCH to linear sRGB channels, unclamped (Björn Ottosson's OKLab matrices). */
function oklchToLinearRgb(lightness: number, chroma: number, hue: number): number[] {
  const radians = (hue * Math.PI) / 180
  const labA = chroma * Math.cos(radians)
  const labB = chroma * Math.sin(radians)
  const l = (lightness + 0.3963377774 * labA + 0.2158037573 * labB) ** 3
  const m = (lightness - 0.1055613458 * labA - 0.0638541728 * labB) ** 3
  const s = (lightness - 0.0894841775 * labA - 1.291485548 * labB) ** 3
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ]
}

function gammaEncode(linear: number): number {
  return linear <= 0.0031308 ? 12.92 * linear : 1.055 * linear ** (1 / 2.4) - 0.055
}

function gammaDecode(encoded: number): number {
  return encoded <= 0.04045 ? encoded / 12.92 : ((encoded + 0.055) / 1.055) ** 2.4
}

function toHexByte(channel: number): string {
  return Math.round(channel * 255)
    .toString(16)
    .padStart(2, '0')
}
