import { test, expect } from '../helpers/fixtures.js'
import { existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { PLAYLIST } from '../helpers/fakeYoutube.js'
import { libraryPath } from '../helpers/library.js'
import { addPlaylist } from '../helpers/seed.js'
import { enterWatching } from '../helpers/video.js'

/** The media files in the playlist's only season folder. */
function episodeMedia(showPath) {
  const season = libraryPath(join(showPath, 'Season 1'))
  return existsSync(season) ? readdirSync(season).filter((name) => name.endsWith('.mp4')) : []
}

test('Discard removes a downloaded video from the list and disk, and restoring brings it back', async ({ page }) => {
  const playlist = await addPlaylist(page.request, PLAYLIST)
  const videosUrl = `/api/playlists/${encodeURIComponent(playlist.id)}/videos`
  expect(episodeMedia(playlist.path)).toHaveLength(playlist.videos.length)

  await page.goto(`/playlists/${encodeURIComponent(playlist.id)}`)
  await enterWatching(page)
  const main = page.locator('main')
  const title = await main.getByRole('heading', { level: 3 }).first().textContent()

  await main.getByRole('button', { name: 'Discard', exact: true }).click()
  const dialog = page.getByRole('alertdialog').or(page.getByRole('dialog'))
  await expect(
    dialog.getByText(
      "Its files will be deleted and it won't be downloaded. You can restore it from its channel's or playlist's menu.",
    ),
  ).toBeVisible()
  const discarded = page.waitForResponse(
    (response) => response.request().method() === 'POST' && response.url().endsWith('/discard'),
  )
  await dialog.getByRole('button', { name: 'Discard', exact: true }).click()
  expect((await discarded).ok()).toBe(true)

  await expect
    .poll(async () => (await (await page.request.get(videosUrl)).json()).map((video) => video.title), {
      message: 'the discarded video no longer listed',
      timeout: 10_000,
    })
    .not.toContain(title)
  await expect(main.getByRole('button').filter({ hasText: title })).toHaveCount(0)
  await expect
    .poll(() => episodeMedia(playlist.path).length, { message: 'its episode deleted from disk', timeout: 30_000 })
    .toBe(playlist.videos.length - 1)

  await page.getByRole('button', { name: /^More actions for / }).click()
  await page.getByRole('menuitem', { name: 'Restore discarded videos' }).click()
  const restoreDialog = page.getByRole('dialog')
  const restored = page.waitForResponse(
    (response) => response.request().method() === 'POST' && response.url().endsWith('/discarded/restore'),
  )
  await restoreDialog.getByRole('button', { name: 'Restore' }).click()
  expect((await restored).status()).toBe(204)

  await expect
    .poll(async () => (await (await page.request.get(videosUrl)).json()).map((video) => video.title), {
      message: 'the restored video listed again',
      timeout: 10_000,
    })
    .toContain(title)
  await expect
    .poll(() => episodeMedia(playlist.path).length, { message: 'its episode downloaded again', timeout: 30_000 })
    .toBe(playlist.videos.length)
})
