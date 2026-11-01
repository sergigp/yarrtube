/**
 * Converts a string into a filesystem-safe slug: NFC-normalized, lowercased,
 * with runs of characters that are not letters, marks or digits (in any
 * script, so `НаучныйСон` stays `научныйсон`) collapsed to a single `-`, and
 * leading/trailing `-` trimmed.
 */
export function slugify(value: string | null | undefined): string {
  return (value ?? '')
    .normalize('NFC')
    .toLowerCase()
    .replace(/[^\p{L}\p{M}\p{N}]+/gu, '-')
    .replace(/^-+|-+$/g, '')
}
