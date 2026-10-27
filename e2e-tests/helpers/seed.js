import { expect } from '@playwright/test'
import { slugOf } from './addDialog.js'

/**
 * Adds a fake-YouTube playlist (see `fakeYoutube.js`) through the API, with
 * the Add dialog's body, and by default waits until its videos are
 * downloaded and nothing more is in flight (about a second against the fake
 * YouTube). Returns it as the API lists it, with its videos.
 */
export async function addPlaylist(request, playlist, { downloaded = true } = {}) {
  const created = await request.post('/api/playlists', {
    data: {
      playlist: playlist.id,
      path: `playlists/${slugOf(playlist.title)}`,
      quality: 'low',
      exclude_from_home: false,
      reconcile_interval_minutes: 180,
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
  const { id, name, path } = await created.json()
  const videosUrl = `/api/playlists/${encodeURIComponent(id)}/videos`
  if (downloaded) {
    await waitUntilSettled(request, { listUrl: '/api/playlists', id, videosUrl, count: playlist.videos.length })
  }
  const videos = await (await request.get(videosUrl)).json()
  return { id, name, path, videos }
}

/** As `addPlaylist`, for the fake-YouTube channel's `videoLimit` newest videos. */
export async function addChannel(request, channel, { videoLimit, downloaded = true }) {
  const created = await request.post('/api/channels', {
    data: {
      channel: channel.handle,
      quality: 'low',
      video_limit: videoLimit,
      path: `channels/${slugOf(channel.handle)}`,
      reconcile_interval_minutes: 180,
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
  const { name, path } = await created.json()
  const videosUrl = `/api/channels/${encodeURIComponent(channel.handle)}/videos`
  if (downloaded) {
    await waitUntilSettled(request, { listUrl: '/api/channels', id: channel.handle, videosUrl, count: videoLimit })
  }
  const videos = await (await request.get(videosUrl)).json()
  return { handle: channel.handle, name, path, videos }
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
      { message: `${id} downloaded and settled`, timeout: 30_000 },
    )
    .toBe(true)
}

// YouTube treats `@Name` and `@name` as the same channel.
function sameHandle(a, b) {
  return a.toLowerCase() === b.toLowerCase()
}
