import type { Category, CategoryMembers } from '@/api/types'

/** A channel or playlist, as a category member. */
export type SourceRef = { kind: 'channel' | 'playlist'; id: string }

/** Unread first, most unread first, then by name ignoring case. */
export function orderCategories(categories: Category[]): Category[] {
  return [...categories].sort(
    (a, b) =>
      b.unwatched_count - a.unwatched_count ||
      a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }),
  )
}

/** How many channels and playlists the category holds. */
export function memberCount(_category: Category): number {
  return 0
}

/** The ids of the categories `source` is a member of. */
export function categoryIdsOf(_categories: Category[], _source: SourceRef): string[] {
  return []
}

/** The category's members with `source` added. */
export function withMember(_category: Category, _source: SourceRef): CategoryMembers {
  return { channels: [], playlists: [] }
}

/** The category's members with `source` removed. */
export function withoutMember(_category: Category, _source: SourceRef): CategoryMembers {
  return { channels: [], playlists: [] }
}

/** The category's members with every one of `sources` added. */
export function withMembers(_category: Category, _sources: SourceRef[]): CategoryMembers {
  return { channels: [], playlists: [] }
}

/** The members PUT each category needs so `source` is in exactly `chosenIds`. */
export function membershipChanges(
  _categories: Category[],
  _source: SourceRef,
  _chosenIds: string[],
): { id: string; members: CategoryMembers }[] {
  return []
}
