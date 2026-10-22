import { test, expect } from '../helpers/fixtures.js'
import { slugOf } from '../helpers/addDialog.js'
import { expectEpisodeOnDisk, libraryPath } from '../helpers/library.js'
import { expectPlexScanned, resetPlexStub } from '../helpers/plex.js'
import { assertVideoPlays, waitForVideoStatus } from '../helpers/video.js'
import { existsSync } from 'node:fs'

const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
const PLAYLIST_NAME = process.env.SMOKE_PLAYLIST_NAME ?? 'test'
const PLAYLIST_PATH = `playlists/${slugOf(PLAYLIST_NAME)}`
const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'
const CHANNEL_PATH = `channels/${slugOf(CHANNEL_HANDLE)}`

/**
 * Waits until the source `id` is settled by the server's own rule, the one a
 * move checks: its `listUrl` entry no longer reports `work_in_progress` (a
 * video still downloading, a per-video task queued in its show folder, or a
 * downloaded video whose event isn't handled yet), and its videos are
 * downloaded with thumbnails, so the page shows one to play.
 */
async function waitUntilSettled(request, { listUrl, id, videosUrl }) {
  await expect
    .poll(
      async () => {
        const sources = await (await request.get(listUrl)).json()
        const videos = await (await request.get(videosUrl)).json()
        const source = sources.find((candidate) => candidate.id === id)
        const downloaded =
          videos.length > 0 &&
          videos.every((video) => video.status === 'DOWNLOADED' && video.thumbnail_filename)
        return source?.work_in_progress === false && downloaded
      },
      { message: `${id} settled`, timeout: 240_000 },
    )
    .toBe(true)
}

/**
 * Opens the source page's edit dialog, checks its Location, and moves it to
 * the folder `newName` next to it through "Move…".
 */
async function moveThroughEditDialog(page, { currentPath, newName, kind }) {
  await page.getByRole('button', { name: /^More actions for / }).click()
  await page.getByRole('menuitem', { name: 'Edit settings' }).click()
  const editDialog = page.getByRole('dialog', { name: /^Edit .+ settings$/ })
  await expect(editDialog.getByText(currentPath, { exact: true })).toBeVisible()
  await editDialog.getByRole('button', { name: 'Move…' }).click()

  const moveDialog = page.getByRole('dialog', { name: /^Move / })
  await expect(editDialog).toBeHidden()
  await expect(moveDialog.getByLabel('Folder name')).toHaveValue(currentPath.split('/').pop())
  await moveDialog.getByLabel('Folder name').fill(newName)
  const moved = page.waitForResponse(
    (response) =>
      response.request().method() === 'POST' &&
      response.url().includes(`/api/${kind}s/`) &&
      response.url().endsWith('/move'),
  )
  await moveDialog.getByRole('button', { name: 'Move', exact: true }).click()
  const response = await moved
  expect(response.ok(), await response.text()).toBe(true)
  await expect(moveDialog).toBeHidden()
}

test('moving a playlist renames its show folder and keeps its videos', async ({ page }) => {
  const newPath = `${PLAYLIST_PATH}-moved`
  const created = await page.request.post('/api/playlists', {
    data: {
      playlist: PLAYLIST_ID,
      path: PLAYLIST_PATH,
      quality: 'low',
      exclude_from_home: false,
      reconcile_interval_minutes: 180,
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
  const playlist = await created.json()
  await waitUntilSettled(page.request, {
    listUrl: '/api/playlists',
    id: playlist.id,
    videosUrl: `/api/playlists/${playlist.id}/videos`,
  })
  await resetPlexStub(page.request)

  await page.goto(`/playlists/${playlist.id}`)
  const videoTitle = await page.locator('main h3').first().innerText()
  await moveThroughEditDialog(page, {
    currentPath: PLAYLIST_PATH,
    newName: newPath.split('/').pop(),
    kind: 'playlist',
  })

  await expectEpisodeOnDisk({ showPath: newPath, kind: 'playlist', showTitle: PLAYLIST_NAME, videoTitle })
  expect(existsSync(libraryPath(PLAYLIST_PATH)), PLAYLIST_PATH).toBe(false)
  await expectPlexScanned(page.request, newPath)
  await expectPlexScanned(page.request, PLAYLIST_PATH)
  await page.reload()
  await waitForVideoStatus(page, { status: 'DOWNLOADED' })
  await assertVideoPlays(page)
})

test('moving a channel renames its show folder and keeps its videos', async ({ page }) => {
  const newPath = `${CHANNEL_PATH}-moved`
  const created = await page.request.post('/api/channels', {
    data: {
      channel: CHANNEL_HANDLE,
      quality: 'low',
      video_limit: 1,
      path: CHANNEL_PATH,
      reconcile_interval_minutes: 180,
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
  const channel = await created.json()
  await waitUntilSettled(page.request, {
    listUrl: '/api/channels',
    id: channel.id,
    videosUrl: `/api/channels/${encodeURIComponent(channel.id)}/videos`,
  })
  await resetPlexStub(page.request)

  await page.goto(`/channels/${encodeURIComponent(channel.id)}`)
  const videoTitle = await page.locator('main h3').first().innerText()
  await moveThroughEditDialog(page, {
    currentPath: CHANNEL_PATH,
    newName: newPath.split('/').pop(),
    kind: 'channel',
  })

  await expectEpisodeOnDisk({ showPath: newPath, kind: 'channel', showTitle: channel.name, videoTitle })
  expect(existsSync(libraryPath(CHANNEL_PATH)), CHANNEL_PATH).toBe(false)
  await expectPlexScanned(page.request, newPath)
  await expectPlexScanned(page.request, CHANNEL_PATH)
  await page.reload()
  await waitForVideoStatus(page, { status: 'DOWNLOADED' })
  await assertVideoPlays(page)
})
