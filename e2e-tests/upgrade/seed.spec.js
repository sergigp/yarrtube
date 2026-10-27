import { test, expect } from '../helpers/fixtures.js'
import {
  PLAYLIST_ID,
  PLAYLIST_PATH,
  POSITION_SECONDS,
  RECONCILE_INTERVAL_MINUTES,
} from './seeded.js'

// Runs against the released image. Drives the API rather than the UI, since
// the released UI may differ from this build's and only its data matters.
// The body is the Add dialog's, every field included: the API requires them.
test('the released image builds up a library to upgrade', async ({ request }) => {
  const created = await request.post('/api/playlists', {
    data: {
      playlist: PLAYLIST_ID,
      path: PLAYLIST_PATH,
      quality: 'low',
      exclude_from_home: false,
      reconcile_interval_minutes: RECONCILE_INTERVAL_MINUTES,
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
  // The released image took every setting, so a mismatch after the upgrade
  // is the upgrade's doing.
  expect(await created.json()).toEqual(
    expect.objectContaining({
      id: PLAYLIST_ID,
      path: PLAYLIST_PATH,
      reconcile_interval_minutes: RECONCILE_INTERVAL_MINUTES,
    }),
  )

  const videosPath = `/api/playlists/${encodeURIComponent(PLAYLIST_ID)}/videos`
  await expect
    .poll(
      async () => {
        const statuses = (await (await request.get(videosPath)).json()).map((video) => video.status)
        return statuses.length > 0 && statuses.every((status) => status === 'DOWNLOADED')
      },
      { message: 'the released image downloads the whole playlist', timeout: 240_000 },
    )
    .toBe(true)

  const [video] = await (await request.get(videosPath)).json()
  const progress = await request.post(`/api/videos/${encodeURIComponent(video.id)}/progress`, {
    data: {
      position_seconds: POSITION_SECONDS,
      duration_seconds: video.duration_seconds,
      was_watched: false,
    },
  })
  expect(progress.ok(), await progress.text()).toBeTruthy()
})
