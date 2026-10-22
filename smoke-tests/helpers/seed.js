import { expect } from '@playwright/test'
import { deleteAllSources } from './fixtures.js'
import { expectGoneFromDisk } from './library.js'
import { slugOf } from './addDialog.js'

const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
const PLAYLIST_PATH = `playlists/${slugOf(process.env.SMOKE_PLAYLIST_NAME ?? 'test')}`
const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'
const CHANNEL_PATH = `channels/${slugOf(CHANNEL_HANDLE)}`

/**
 * Tracks the fixture playlist through the API (with the Add dialog's body,
 * at low quality to keep the download quick) on an empty library, and waits
 * until its video is downloaded and nothing more is in flight for it. For a
 * spec file's `beforeAll`, with `test.use({ keepLibrary: true })`, so the
 * file's tests share one download yet stay independent of each other.
 */
export async function seedDownloadedPlaylist(request) {
  await deleteAllSources(request)
  const created = await request.post('/api/playlists', {
    data: {
      playlist: PLAYLIST_ID,
      path: PLAYLIST_PATH,
      quality: 'low',
      exclude_from_home: false,
      reconcile_interval_minutes: 180,
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
  const { id, name, path } = await created.json()
  const videosUrl = `/api/playlists/${encodeURIComponent(id)}/videos`
  await waitUntilSettled(request, { listUrl: '/api/playlists', id, videosUrl, count: 1 })
  const [video] = await (await request.get(videosUrl)).json()
  return { id, name, path, video }
}

/** As `seedDownloadedPlaylist`, for the fixture channel's latest `videoLimit` videos. */
export async function seedDownloadedChannel(request, { videoLimit }) {
  await deleteAllSources(request)
  const created = await request.post('/api/channels', {
    data: {
      channel: CHANNEL_HANDLE,
      quality: 'low',
      video_limit: videoLimit,
      path: CHANNEL_PATH,
      reconcile_interval_minutes: 180,
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
  const videosUrl = `/api/channels/${encodeURIComponent(CHANNEL_HANDLE)}/videos`
  await waitUntilSettled(request, {
    listUrl: '/api/channels',
    id: CHANNEL_HANDLE,
    videosUrl,
    count: videoLimit,
  })
  const channels = await (await request.get('/api/channels')).json()
  const { name, path } = channels.find((channel) => sameHandle(channel.id, CHANNEL_HANDLE))
  const videos = await (await request.get(videosUrl)).json()
  return { handle: CHANNEL_HANDLE, name, path, videos }
}

/** Deletes what `seedDownloaded…` tracked and waits for its folder to go. */
export async function deleteSeeded(request, seeded) {
  const url = seeded.handle
    ? `/api/channels/${encodeURIComponent(seeded.handle)}`
    : `/api/playlists/${encodeURIComponent(seeded.id)}`
  const deleted = await request.delete(url)
  expect(deleted.ok(), `DELETE ${url}`).toBeTruthy()
  await expectGoneFromDisk(seeded.path)
}

/**
 * Waits until the source `id` has `count` videos downloaded with thumbnails
 * and its `listUrl` entry no longer reports `work_in_progress` (subtitles,
 * Plex scans and the like still running).
 */
async function waitUntilSettled(request, { listUrl, id, videosUrl, count }) {
  await expect
    .poll(
      async () => {
        const sources = await (await request.get(listUrl)).json()
        const videos = await (await request.get(videosUrl)).json()
        const source = sources.find((candidate) => sameHandle(candidate.id, id))
        const downloaded =
          videos.length === count &&
          videos.every((video) => video.status === 'DOWNLOADED' && video.thumbnail_filename)
        return source?.work_in_progress === false && downloaded
      },
      { message: `${id} downloaded and settled`, timeout: 240_000 },
    )
    .toBe(true)
}

// YouTube treats `@Name` and `@name` as the same channel.
function sameHandle(a, b) {
  return a.toLowerCase() === b.toLowerCase()
}
