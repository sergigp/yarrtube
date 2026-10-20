import type { Category, CategoryMembers, ChannelListItem, PlaylistListItem } from '@/api/types'

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

/** `items` by name ignoring case; a missing list is empty. */
export function byName<T extends { name: string }>(items: T[] | undefined): T[] {
  return [...(items ?? [])].sort((a, b) =>
    a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }),
  )
}

/** How many channels and playlists the category holds. */
export function memberCount(category: Category): number {
  return category.channels.length + category.playlists.length
}

/** The delete confirmation's reassurance: what the category holds stays. */
export function deleteCategoryDescription(category: Category): string {
  return `"${category.name}" holds ${memberCount(category)} channels and playlists. None of them, nor any video, will be deleted.`
}

/** The ids of the categories `source` is a member of. */
export function categoryIdsOf(categories: Category[], source: SourceRef): string[] {
  return categories.filter((category) => isMember(category, source)).map((category) => category.id)
}

/** Whether `source` is a member of the category. */
export function isMember(category: Category, source: SourceRef): boolean {
  return membersOfKind(category, source.kind).includes(source.id)
}

/** The category's members with `source` added. */
export function withMember(category: Category, source: SourceRef): CategoryMembers {
  return withMembers(category, [source])
}

/** The category's members with `source` removed. */
export function withoutMember(category: Category, source: SourceRef): CategoryMembers {
  return replaceMembersOfKind(
    category,
    source.kind,
    membersOfKind(category, source.kind).filter((id) => id !== source.id),
  )
}

/** The category's members with every one of `sources` added. */
export function withMembers(category: Category, sources: SourceRef[]): CategoryMembers {
  const added = (kind: SourceRef['kind']) =>
    sources.filter((source) => source.kind === kind).map((source) => source.id)
  return {
    channels: unique([...category.channels, ...added('channel')]),
    playlists: unique([...category.playlists, ...added('playlist')]),
  }
}

/** The members PUT each category needs so `source` is in exactly `chosenIds`. */
export function membershipChanges(
  categories: Category[],
  source: SourceRef,
  chosenIds: string[],
): { id: string; members: CategoryMembers }[] {
  const memberOf = categoryIdsOf(categories, source)
  return categories.flatMap((category) => {
    const chosen = chosenIds.includes(category.id)
    if (chosen === memberOf.includes(category.id)) {
      return []
    }
    const members = chosen ? withMember(category, source) : withoutMember(category, source)
    return [{ id: category.id, members }]
  })
}

/** A tracked channel or playlist as a chip of the manage sources dialog. */
export interface SourceChip {
  source: SourceRef
  name: string
  avatarFilename: string | null
}

/** Which chips the manage sources dialog shows, by whether they're in the category. */
export type SourceFilter = 'all' | 'in' | 'out'

/** Every tracked source as a chip, each kind by name ignoring case. */
export function sourceChips(
  channels: ChannelListItem[] | undefined,
  playlists: PlaylistListItem[] | undefined,
): { channels: SourceChip[]; playlists: SourceChip[] } {
  return {
    channels: byName(channels).map((channel) => ({
      source: { kind: 'channel', id: channel.id },
      name: channel.name,
      avatarFilename: channel.avatar_filename,
    })),
    playlists: byName(playlists).map((playlist) => ({
      source: { kind: 'playlist', id: playlist.id },
      name: playlist.name,
      avatarFilename: null,
    })),
  }
}

/** The category's members, as the manage sources dialog stages them. */
export function membersOf(_category: Category): CategoryMembers {
  return { channels: [], playlists: [] }
}

/** Whether `source` is among `members`. */
export function isIn(_members: CategoryMembers, _source: SourceRef): boolean {
  return false
}

/** `members` with `source` added when absent, removed when present. */
export function toggled(members: CategoryMembers, _source: SourceRef): CategoryMembers {
  return members
}

/** Chips matching the search (name, ignoring case) and the state filter, order kept. */
export function visibleChips(
  chips: SourceChip[],
  _staged: CategoryMembers,
  _search: string,
  _filter: SourceFilter,
): SourceChip[] {
  return chips
}

/** How many sources `staged` adds to and removes from `saved`. */
export function stagedSummary(
  _saved: CategoryMembers,
  _staged: CategoryMembers,
): { added: number; removed: number } {
  return { added: 0, removed: 0 }
}

function membersOfKind(category: Category, kind: SourceRef['kind']): string[] {
  return kind === 'channel' ? category.channels : category.playlists
}

function replaceMembersOfKind(
  category: Category,
  kind: SourceRef['kind'],
  ids: string[],
): CategoryMembers {
  return kind === 'channel'
    ? { channels: ids, playlists: category.playlists }
    : { channels: category.channels, playlists: ids }
}

function unique(ids: string[]): string[] {
  return [...new Set(ids)]
}
