import { test, expect } from '../helpers/fixtures.js'
import { slugOf } from '../helpers/addDialog.js'
import { libraryPath } from '../helpers/library.js'
import { existsSync } from 'node:fs'
import { join } from 'node:path'

const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'
const CHANNEL_PATH = `channels/${slugOf(CHANNEL_HANDLE)}`
const VIDEOS_URL = `/api/channels/${encodeURIComponent(CHANNEL_HANDLE)}/videos`

/** The host paths of a downloaded video's media, `.nfo` and thumbnail. */
function filesOf(video) {
  const media = join(libraryPath(CHANNEL_PATH), video.filename)
  return [media, media.replace(/\.[^.]+$/, '.nfo'), join(libraryPath(CHANNEL_PATH), video.thumbnail_filename)]
}

test('lowering a channel video limit deletes the videos beyond it on the next sync', async ({ page }) => {
  // Set up through the API with the Add dialog's body (at low quality, to
  // keep two downloads quick): what is under test is the edit and the sync
  // after it.
  const created = await page.request.post('/api/channels', {
    data: {
      channel: CHANNEL_HANDLE,
      quality: 'low',
      video_limit: 2,
      path: CHANNEL_PATH,
      reconcile_interval_minutes: 180,
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
  await expect
    .poll(
      async () => {
        const videos = await (await page.request.get(VIDEOS_URL)).json()
        return videos.filter((video) => video.status === 'DOWNLOADED' && video.thumbnail_filename).length
      },
      { message: 'both videos downloaded with thumbnails', timeout: 240_000 },
    )
    .toBe(2)
  const [newest, oldest] = (await (await page.request.get(VIDEOS_URL)).json()).sort((a, b) =>
    b.published_at.localeCompare(a.published_at),
  )
  for (const file of [...filesOf(newest), ...filesOf(oldest)]) {
    expect(existsSync(file), file).toBe(true)
  }

  // Lower the limit to 1 from the channel page's settings; the dialog warns
  // what the next sync will do.
  await page.goto(`/channels/${encodeURIComponent(CHANNEL_HANDLE)}`)
  await page.getByRole('button', { name: /^More actions for / }).click()
  await page.getByRole('menuitem', { name: 'Edit settings' }).click()
  const editDialog = page.getByRole('dialog', { name: /^Edit .+ settings$/ })
  await editDialog.getByLabel('Video limit').fill('1')
  await expect(editDialog.getByText(/The next sync deletes the downloaded videos/)).toBeVisible()
  const saved = page.waitForResponse(
    (response) => response.request().method() === 'PATCH' && response.url().includes('/channels/'),
  )
  await editDialog.getByRole('button', { name: 'Save' }).click()
  expect((await saved).ok()).toBe(true)
  await editDialog.waitFor({ state: 'hidden' })

  // Sync from the page header: the oldest video goes, from the list and
  // from disk, and the newest stays.
  const synced = page.waitForResponse(
    (response) => response.url().endsWith('/reconcile') && response.request().method() === 'POST',
  )
  await page.getByRole('button', { name: 'Sync', exact: true }).click()
  expect((await synced).ok()).toBe(true)
  // The header counts the videos ("1 video · 1 unwatched").
  await expect(page.locator('main').getByText(/^1 video\b/)).toBeVisible({ timeout: 30_000 })
  await expect(page.locator('main').getByText(oldest.title, { exact: true })).toHaveCount(0)
  await expect(page.locator('main').getByText(newest.title, { exact: true }).first()).toBeVisible()
  await expect
    .poll(() => filesOf(oldest).filter((file) => existsSync(file)), { timeout: 30_000 })
    .toEqual([])
  for (const file of filesOf(newest)) {
    expect(existsSync(file), file).toBe(true)
  }
})
