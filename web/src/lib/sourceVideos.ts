import type { HomeVideo, HomeVideoSource, Video } from '@/api/types'

/** What a video list needs to filter, sort and pick by watch state. */
export interface WatchableVideo {
  id: string
  watched: boolean
  position_seconds: number
  published_at: string | null
}

/** The watch-state chips over a channel, playlist or category's videos. */
export type WatchFilter = 'all' | 'unwatched' | 'in_progress' | 'watched'

export const WATCH_FILTERS: readonly WatchFilter[] = ['all', 'unwatched', 'in_progress', 'watched']

export const WATCH_FILTER_LABELS: Record<WatchFilter, string> = {
  all: 'All',
  unwatched: 'Unwatched',
  in_progress: 'In progress',
  watched: 'Watched',
}

/** How a video list is ordered: as the playlist lists it, or by publish date. */
export type VideoSort = 'playlist' | 'newest' | 'oldest'

export const VIDEO_SORT_LABELS: Record<VideoSort, string> = {
  playlist: 'Playlist order',
  newest: 'Newest first',
  oldest: 'Oldest first',
}

/** Started but not finished: a subset of unwatched. */
export function isInProgress(video: WatchableVideo): boolean {
  return !video.watched && video.position_seconds > 0
}

export function matchesWatchFilter(video: WatchableVideo, filter: WatchFilter): boolean {
  switch (filter) {
    case 'all':
      return true
    case 'unwatched':
      return !video.watched
    case 'in_progress':
      return isInProgress(video)
    case 'watched':
      return video.watched
  }
}

/** How many of `videos` each chip would show. */
export function watchFilterCounts(videos: readonly WatchableVideo[]): Record<WatchFilter, number> {
  return {
    all: videos.length,
    unwatched: videos.filter((video) => matchesWatchFilter(video, 'unwatched')).length,
    in_progress: videos.filter((video) => matchesWatchFilter(video, 'in_progress')).length,
    watched: videos.filter((video) => matchesWatchFilter(video, 'watched')).length,
  }
}

/**
 * `videos` in `sort` order: `playlist` keeps the order given (the API lists
 * a playlist in playlist order, a channel or category newest first); by date,
 * videos without a publish date go last, ties keep their given order.
 */
export function sortVideos<T extends WatchableVideo>(videos: readonly T[], sort: VideoSort): T[] {
  if (sort === 'playlist') {
    return [...videos]
  }
  const direction = sort === 'newest' ? -1 : 1
  return videos
    .map((video, index) => ({ video, index, time: publishedTime(video) }))
    .sort((a, b) => {
      if (a.time === null || b.time === null) {
        return a.time === b.time ? a.index - b.index : a.time === null ? 1 : -1
      }
      return a.time === b.time ? a.index - b.index : (a.time - b.time) * direction
    })
    .map(({ video }) => video)
}

/**
 * What "Play next" plays: the first video of `videos`, in the order given,
 * not watched yet, whether started or not. Watched 1, 3 and 5 of a playlist
 * plays 2. `null` when every video is watched.
 */
export function nextToPlay<T extends WatchableVideo>(videos: readonly T[]): T | null {
  return videos.find((video) => !video.watched) ?? null
}

/** The newest video not watched yet, by publish date. */
export function newestUnwatched<T extends WatchableVideo>(videos: readonly T[]): T | null {
  return nextToPlay(sortVideos(videos, 'newest'))
}

/** A channel or playlist a category's videos come from, with how many it has. */
export interface VideoSourceOption {
  key: string
  kind: HomeVideoSource['kind']
  name: string
  avatarFilename: string | null
  count: number
}

/** Identifies a video's source across channels and playlists. */
export function sourceKey(source: Pick<HomeVideoSource, 'kind' | 'id'>): string {
  return `${source.kind}:${source.id}`
}

/** Every source among `videos`, the one with the most videos first, then by name. */
export function videoSources(
  videos: readonly { source: HomeVideoSource }[],
): VideoSourceOption[] {
  const byKey = new Map<string, VideoSourceOption>()
  for (const { source } of videos) {
    const key = sourceKey(source)
    const known = byKey.get(key)
    byKey.set(
      key,
      known
        ? { ...known, count: known.count + 1 }
        : {
            key,
            kind: source.kind,
            name: source.name,
            avatarFilename: source.avatar_filename,
            count: 1,
          },
    )
  }
  return [...byKey.values()].sort(
    (a, b) => b.count - a.count || a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }),
  )
}

/** "from 3 channels and playlists", or the one kind when they're all alike. */
export function sourcesSummary(sources: readonly Pick<VideoSourceOption, 'kind'>[]): string | null {
  if (sources.length === 0) {
    return null
  }
  const channels = sources.filter((source) => source.kind === 'channel').length
  const count = sources.length
  if (channels === count) {
    return `from ${count} ${count === 1 ? 'channel' : 'channels'}`
  }
  if (channels === 0) {
    return `from ${count} ${count === 1 ? 'playlist' : 'playlists'}`
  }
  return `from ${count} channels and playlists`
}

/**
 * The meta line under a source's name, e.g. "Playlist · 32 videos · 29
 * unwatched · synced 14d ago", dropping the unwatched part when nothing is
 * and the sync part until a sync is known.
 */
export function sourceSummary({
  kind,
  videoCount,
  unwatchedCount,
  syncedAgo,
}: {
  kind?: string
  videoCount: number
  unwatchedCount?: number
  syncedAgo?: string | null
}): string {
  const parts = [`${videoCount} ${videoCount === 1 ? 'video' : 'videos'}`]
  if (kind) {
    parts.unshift(kind)
  }
  if (unwatchedCount !== undefined && unwatchedCount > 0) {
    parts.push(`${unwatchedCount} unwatched`)
  }
  if (syncedAgo) {
    parts.push(`synced ${syncedAgo}`)
  }
  return parts.join(' · ')
}

function publishedTime(video: WatchableVideo): number | null {
  const time = video.published_at ? new Date(video.published_at).getTime() : Number.NaN
  return Number.isNaN(time) ? null : time
}

/** A channel or playlist's own video as a card shows it, coming from `source`. */
export function asHomeVideo(video: Video, source: HomeVideoSource): HomeVideo {
  return {
    id: video.id,
    youtube_id: video.youtube_id,
    title: video.title,
    thumbnail_filename: video.thumbnail_filename,
    duration_seconds: video.duration_seconds,
    watched: video.watched,
    position_seconds: video.position_seconds,
    published_at: video.published_at,
    source,
    category_ids: [],
  }
}
