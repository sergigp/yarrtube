import { test, expect } from '../helpers/fixtures.js'
import { assertVideoPlays } from '../helpers/video.js'
import { expectEpisodeOnDisk, libraryPath } from '../helpers/library.js'
import { expectPendingSync } from '../helpers/tasks.js'
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import {
  PLAYLIST_ID,
  PLAYLIST_NAME,
  PLAYLIST_PATH,
  POSITION_SECONDS,
  RECONCILE_INTERVAL_MINUTES,
} from './seeded.js'

// Runs against this build, started on the database and videos the released
// image left behind (`seed.spec.js`), so the library must survive as is.
test.use({ keepLibrary: true })

test('this build carries on the library the released image left behind', async ({ page }) => {
  // The playlist, its settings and its download all migrated.
  const playlists = await (await page.request.get('/api/playlists')).json()
  expect(playlists).toEqual([
    expect.objectContaining({
      id: PLAYLIST_ID,
      name: PLAYLIST_NAME,
      path: PLAYLIST_PATH,
      reconcile_interval_minutes: RECONCILE_INTERVAL_MINUTES,
    }),
  ])
  const [{ path }] = playlists
  const videos = await (
    await page.request.get(`/api/playlists/${encodeURIComponent(PLAYLIST_ID)}/videos`)
  ).json()
  expect(videos.map((video) => video.status)).toEqual(videos.map(() => 'DOWNLOADED'))
  const [video] = videos
  expect(video.position_seconds).toBe(POSITION_SECONDS)
  expect(existsSync(join(libraryPath(path), video.filename)), video.filename).toBe(true)
  await expectEpisodeOnDisk({
    showPath: path,
    kind: 'playlist',
    showTitle: PLAYLIST_NAME,
    videoTitle: video.title,
  })

  // Nothing is downloaded again: the files on disk are taken as they are.
  const tasks = await (await page.request.get('/api/tasks')).json()
  expect(tasks.filter((task) => task.task_type === 'download_video')).toEqual([])

  // Opened straight from its URL, the playlist plays its video.
  await page.goto(`/playlists/${encodeURIComponent(PLAYLIST_ID)}`)
  await expect(page.locator('main h3').first()).toHaveText(video.title)
  await assertVideoPlays(page)

  // The position recorded before the upgrade still offers it to continue.
  await page.goto('/')
  const continueWatching = page.locator('section', {
    has: page.getByRole('heading', { name: 'Continue watching' }),
  })
  await expect(continueWatching.getByText(video.title, { exact: true })).toBeVisible()

  // The next sync is still scheduled from the playlist's own interval.
  await expectPendingSync(page, {
    label: `Syncing playlist ${PLAYLIST_NAME}`,
    due: /^in (3\d|4[0-5])m$/,
  })
})
