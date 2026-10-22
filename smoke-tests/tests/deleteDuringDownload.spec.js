import { test, expect } from '../helpers/fixtures.js'
import { slugOf } from '../helpers/addDialog.js'
import { expectGoneFromDisk, libraryPath } from '../helpers/library.js'
import { deleteItem, sectionRows } from '../helpers/sidebar.js'
import { existsSync } from 'node:fs'

const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
const PLAYLIST_NAME = process.env.SMOKE_PLAYLIST_NAME ?? 'test'
const PLAYLIST_PATH = `playlists/${slugOf(PLAYLIST_NAME)}`

async function downloadTasks(request) {
  const tasks = await (await request.get('/api/tasks')).json()
  return tasks.filter((task) => task.task_type === 'download_video')
}

test('deleting a playlist mid-download leaves no folder', async ({ page }) => {
  // Added through the API at high quality, so the download is still running
  // when the playlist is deleted.
  const created = await page.request.post('/api/playlists', {
    data: {
      playlist: PLAYLIST_ID,
      path: PLAYLIST_PATH,
      quality: 'high',
      exclude_from_home: false,
      reconcile_interval_minutes: 180,
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
  await expect
    .poll(async () => (await downloadTasks(page.request)).map((task) => task.status), {
      message: 'a download running',
      timeout: 60_000,
    })
    .toContain('running')
  await expect.poll(() => existsSync(libraryPath(PLAYLIST_PATH)), { message: 'show folder created', timeout: 60_000 }).toBe(true)

  await page.goto('/')
  await deleteItem(page, { section: 'Playlists', name: PLAYLIST_NAME })
  await expect(sectionRows(page, 'Playlists')).toHaveCount(0)

  await expectGoneFromDisk(PLAYLIST_PATH)
  // The download in flight ends (cancelled or finished) without writing the
  // folder back.
  await expect
    .poll(async () => (await downloadTasks(page.request)).length, {
      message: 'in-flight download over',
      timeout: 240_000,
    })
    .toBe(0)
  // Give a late write the time to land before checking it didn't.
  await page.waitForTimeout(5_000)
  expect(existsSync(libraryPath(PLAYLIST_PATH)), `${PLAYLIST_PATH} reappeared`).toBe(false)
})
