/** Case-insensitive name match; expects `text` already trimmed. */
export function matchesSearch(item: { name: string }, text: string): boolean {
  return item.name.toLowerCase().includes(text.toLowerCase())
}

/** The entries whose name contains `text` once trimmed, ignoring case; none for blank text. */
export function searchByName<T extends { name: string }>(entries: T[], text: string): T[] {
  const trimmed = text.trim()
  return trimmed ? entries.filter((entry) => matchesSearch(entry, trimmed)) : []
}
