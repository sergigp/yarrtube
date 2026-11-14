import type { PlexPlaylistOrder, PlexPlaylistWatched } from '@/api/types'

/** One example episode in the category playlist preview. */
export interface PreviewRow {
  label: string
  watched: boolean
}

/** Example rows showing how a category playlist reacts to its options. */
export function previewRows(_watched: PlexPlaylistWatched, _order: PlexPlaylistOrder): PreviewRow[] {
  return []
}
