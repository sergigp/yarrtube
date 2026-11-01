import type { HomeVideoSource } from '@/api/types'
import { channelPath, playlistPath } from './sourcePath'

/** Where a video opens by default: selected in its channel's or playlist's view. */
export function videoPathInSource(video: { id: string; source: HomeVideoSource }): string {
  const base =
    video.source.kind === 'channel'
      ? channelPath(video.source.id)
      : playlistPath(video.source.id)
  return `${base}?video=${encodeURIComponent(video.id)}`
}
