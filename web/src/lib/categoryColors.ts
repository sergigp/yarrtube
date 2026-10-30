/** The category color presets, in the order defaults are handed out. Mirrors the daemon's palette. */
export const PALETTE: readonly { name: string; hex: string }[] = [
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
]

/** The palette color used by the fewest of `used`, the earliest on a tie. */
export function defaultColor(used: string[]): string {
  const uses = (hex: string) => used.filter((color) => color === hex).length
  return PALETTE.reduce((least, color) => (uses(color.hex) < uses(least.hex) ? color : least)).hex
}

/** The custom hue slider's lightness and chroma, vivid like the palette. */
const CUSTOM = { lightness: 0.65, chroma: 0.2 }
/** Chip and category header backgrounds. */
const SOFT = { lightness: 0.92, chroma: 0.07 }
/** Chip text, readable on the soft tint. */
const INK = { lightness: 0.4, chroma: 0.12 }
/** Chroma from which a color gets the full tint; more muted colors get a fainter one. */
const VIVID_CHROMA = 0.15

/** The color of `hue` (degrees), as the custom hue slider picks it. */
export function solidFromHue(hue: number): string {
  return fromOklch(CUSTOM.lightness, CUSTOM.chroma, hue)
}

/** The soft tint of `hex`'s hue: chip and category header backgrounds. */
export function softTint(hex: string): string {
  return fromOklch(SOFT.lightness, SOFT.chroma * vividness(hex), hueOf(hex))
}

/** The dark shade of `hex`'s hue: chip text, readable on `softTint(hex)`. */
export function inkShade(hex: string): string {
  return fromOklch(INK.lightness, INK.chroma * vividness(hex), hueOf(hex))
}

/** The oklch hue of `hex`, in degrees, from 0 up to 360. */
export function hueOf(hex: string): number {
  const [labA, labB] = toOklab(hex)
  return ((Math.atan2(labB, labA) * 180) / Math.PI + 360) % 360
}

/** How vivid `hex` is, from 0 (gray) up to 1 (at or above `VIVID_CHROMA`). */
function vividness(hex: string): number {
  const [labA, labB] = toOklab(hex)
  return Math.min(1, Math.hypot(labA, labB) / VIVID_CHROMA)
}

/** The OKLab a and b of `hex`. */
function toOklab(hex: string): [number, number] {
  const [r, g, b] = [1, 3, 5].map((at) => gammaDecode(parseInt(hex.slice(at, at + 2), 16) / 255))
  const l = Math.cbrt(0.4122214708 * r + 0.5363329363 * g + 0.0514459929 * b)
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
  return [
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ]
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
