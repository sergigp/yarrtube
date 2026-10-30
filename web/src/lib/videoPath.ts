import type { HomeVideoSource } from '@/api/types'

/** Where a video opens by default: selected in its channel's or playlist's view. */
export function videoPathInSource(video: { id: string; source: HomeVideoSource }): string {
  const base =
    video.source.kind === 'channel'
      ? `/channels/${video.source.id}`
      : `/playlists/${video.source.id}`
  return `${base}?video=${encodeURIComponent(video.id)}`
}
