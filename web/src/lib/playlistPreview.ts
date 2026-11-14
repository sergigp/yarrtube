import type { PlexPlaylistOrder, PlexPlaylistWatched } from '@/api/types'

/** One example episode in the category playlist preview. */
export interface PreviewRow {
  label: string
  watched: boolean
}

/** Example episodes, newest first; the middle one is watched. */
const EXAMPLES: PreviewRow[] = [
  { label: 'Published today', watched: false },
  { label: 'Published last week', watched: true },
  { label: 'Published last month', watched: false },
]

/** Example rows showing how a category playlist reacts to its options. */
export function previewRows(watched: PlexPlaylistWatched, order: PlexPlaylistOrder): PreviewRow[] {
  const kept = watched === 'drop' ? EXAMPLES.filter((row) => !row.watched) : EXAMPLES
  return order === 'oldest_first' ? [...kept].reverse() : kept
}
