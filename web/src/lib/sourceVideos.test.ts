import { describe, expect, it } from 'vitest'
import {
  episodeLabel,
  isInProgress,
  matchesWatchFilter,
  newestUnwatched,
  nextToPlay,
  sortVideos,
  sourceSummary,
  sourcesSummary,
  videoSources,
  watchFilterCounts,
  type WatchableVideo,
} from './sourceVideos'
import type { HomeVideoSource } from '@/api/types'

function video(id: string, overrides: Partial<WatchableVideo> = {}): WatchableVideo {
  return { id, watched: false, position_seconds: 0, in_progress: false, published_at: null, ...overrides }
}

function source(kind: HomeVideoSource['kind'], id: string): { source: HomeVideoSource } {
  return { source: { kind, id, name: id, path: `${kind}s/${id}`, avatar_filename: null } }
}

describe('nextToPlay', () => {
  it('plays the first unwatched video in the order given', () => {
    const videos = [
      video('1', { watched: true }),
      video('2'),
      video('3', { watched: true }),
      video('4'),
      video('5', { watched: true }),
    ]

    expect(nextToPlay(videos)?.id).toBe('2')
  })

  it('plays a started video, which resumes from its saved position', () => {
    const videos = [video('1', { watched: true }), video('2', { position_seconds: 90 })]

    expect(nextToPlay(videos)?.id).toBe('2')
  })

  it('skips a video not downloaded yet', () => {
    const videos = [
      video('failed', { status: 'FAILED' }),
      video('2', { watched: true }),
      video('3', { status: 'DOWNLOADED' }),
    ]

    expect(nextToPlay(videos)?.id).toBe('3')
  })

  it('has nothing to play once every video is watched', () => {
    expect(nextToPlay([video('1', { watched: true })])).toBeNull()
  })
})

describe('newestUnwatched', () => {
  it('plays the newest video not watched yet, by publish date', () => {
    const videos = [
      video('old', { published_at: '2026-01-01T00:00:00Z' }),
      video('newest', { published_at: '2026-03-01T00:00:00Z', watched: true }),
      video('newer', { published_at: '2026-02-01T00:00:00Z', position_seconds: 30 }),
    ]

    expect(newestUnwatched(videos)?.id).toBe('newer')
  })

  it('skips the newest video while it is still downloading', () => {
    const videos = [
      video('downloading', { published_at: '2026-03-01T00:00:00Z', status: 'IN_PROGRESS' }),
      video('ready', { published_at: '2026-02-01T00:00:00Z', status: 'DOWNLOADED' }),
    ]

    expect(newestUnwatched(videos)?.id).toBe('ready')
  })
})

describe('watch filters', () => {
  const videos = [
    video('fresh'),
    video('started', { position_seconds: 10 }),
    video('watched', { watched: true, position_seconds: 10 }),
  ]

  it('counts in progress as a subset of unwatched', () => {
    expect(watchFilterCounts(videos)).toEqual({
      all: 3,
      unwatched: 2,
      in_progress: 1,
      watched: 1,
    })
  })

  it('keeps the videos each filter names', () => {
    const ids = (filter: Parameters<typeof matchesWatchFilter>[1]) =>
      videos.filter((item) => matchesWatchFilter(item, filter)).map((item) => item.id)

    expect([ids('all'), ids('unwatched'), ids('in_progress'), ids('watched')]).toEqual([
      ['fresh', 'started', 'watched'],
      ['fresh', 'started'],
      ['started'],
      ['watched'],
    ])
  })

  it('counts a video not downloaded yet under all only', () => {
    const withFailed = [
      video('ready', { status: 'DOWNLOADED' }),
      video('failed', { status: 'FAILED', position_seconds: 5 }),
    ]

    expect(watchFilterCounts(withFailed)).toEqual({
      all: 2,
      unwatched: 1,
      in_progress: 0,
      watched: 0,
    })
  })

  it('treats a watched video with a saved position as not in progress', () => {
    expect(isInProgress(video('x', { watched: true, position_seconds: 50 }))).toBe(false)
  })
})

describe('sortVideos', () => {
  const videos = [
    video('b', { published_at: '2026-02-01T00:00:00Z' }),
    video('undated'),
    video('c', { published_at: '2026-03-01T00:00:00Z' }),
    video('a', { published_at: '2026-01-01T00:00:00Z' }),
  ]
  const ids = (sorted: WatchableVideo[]) => sorted.map((item) => item.id)

  it('keeps playlist order as given', () => {
    expect(ids(sortVideos(videos, 'playlist'))).toEqual(['b', 'undated', 'c', 'a'])
  })

  it('sorts by publish date either way, undated videos last', () => {
    expect([ids(sortVideos(videos, 'newest')), ids(sortVideos(videos, 'oldest'))]).toEqual([
      ['c', 'b', 'a', 'undated'],
      ['a', 'b', 'c', 'undated'],
    ])
  })
})

describe('videoSources', () => {
  it('lists each source once with its video count, the biggest first', () => {
    const videos = [
      source('channel', 'Zed'),
      source('playlist', 'Alpha'),
      source('channel', 'Zed'),
      source('channel', 'Beta'),
    ]

    expect(videoSources(videos)).toEqual([
      { key: 'channel:Zed', kind: 'channel', name: 'Zed', avatarFilename: null, count: 2 },
      { key: 'playlist:Alpha', kind: 'playlist', name: 'Alpha', avatarFilename: null, count: 1 },
      { key: 'channel:Beta', kind: 'channel', name: 'Beta', avatarFilename: null, count: 1 },
    ])
  })
})

describe('sourcesSummary', () => {
  it('names the kinds of sources the videos come from', () => {
    expect([
      sourcesSummary([]),
      sourcesSummary([{ kind: 'channel' }]),
      sourcesSummary([{ kind: 'playlist' }, { kind: 'playlist' }]),
      sourcesSummary([{ kind: 'channel' }, { kind: 'playlist' }]),
    ]).toEqual([
      null,
      'from 1 channel',
      'from 2 playlists',
      'from 2 channels and playlists',
    ])
  })
})

describe('sourceSummary', () => {
  it('joins the kind, counts and last sync', () => {
    expect(
      sourceSummary({ kind: 'Playlist', videoCount: 32, unwatchedCount: 29, syncedAgo: '14d ago' }),
    ).toBe('Playlist · 32 videos · 29 unwatched · synced 14d ago')
  })

  it('drops what is unknown or zero and singularizes one video', () => {
    expect(sourceSummary({ videoCount: 1, unwatchedCount: 0, syncedAgo: null })).toBe('1 video')
  })
})

describe('episodeLabel', () => {
  it('names the episode short or long, and nothing without one', () => {
    expect([episodeLabel(4), episodeLabel(4, 'long'), episodeLabel(null)]).toEqual([
      'Ep 4',
      'Episode 4',
      null,
    ])
  })
})
