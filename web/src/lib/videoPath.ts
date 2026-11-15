import type { HomeVideoSource } from '@/api/types'
import { channelPath, playlistPath } from './sourcePath'

/** The page at `pagePath` (a channel, playlist or category) playing the video `videoId`. */
export function videoPathIn(pagePath: string, videoId: string): string {
  return `${pagePath}?video=${encodeURIComponent(videoId)}`
}

/** Where a video opens by default: selected in its channel's or playlist's view. */
export function videoPathInSource(video: { id: string; source: HomeVideoSource }): string {
  const base =
    video.source.kind === 'channel'
      ? channelPath(video.source.id)
      : playlistPath(video.source.id)
  return videoPathIn(base, video.id)
}
