import { test, expect } from '../helpers/fixtures.js'
import { slugOf } from '../helpers/addDialog.js'
import { expectGoneFromDisk, libraryPath } from '../helpers/library.js'
import { deleteItem, sectionRows } from '../helpers/sidebar.js'
import { existsSync } from 'node:fs'
import { SLOW_PLAYLIST } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'

const PLAYLIST_NAME = SLOW_PLAYLIST.title
const PLAYLIST_PATH = `playlists/${slugOf(PLAYLIST_NAME)}`

async function downloadTasks(request) {
  const tasks = await (await request.get('/api/tasks')).json()
  return tasks.filter((task) => task.task_type === 'download_video')
}

test('deleting a playlist mid-download leaves no folder', async ({ page }) => {
  // The slow fixture video takes about 8 s to download, so it is still
  // running when the playlist is deleted.
  await addPlaylist(page.request, SLOW_PLAYLIST, { downloaded: false })
  await expect
    .poll(async () => (await downloadTasks(page.request)).map((task) => task.status), {
      message: 'a download running',
      timeout: 10_000,
    })
    .toContain('running')
  await expect.poll(() => existsSync(libraryPath(PLAYLIST_PATH)), { message: 'show folder created', timeout: 10_000 }).toBe(true)

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
