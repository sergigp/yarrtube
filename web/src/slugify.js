/**
 * Converts a string into a filesystem-safe slug: lowercased, with runs of
 * non-alphanumeric characters collapsed to a single `-`, and leading/trailing
 * `-` trimmed.
 */
export function slugify(value) {
  return (value ?? '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}
