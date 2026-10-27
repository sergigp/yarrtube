import { test, expect } from '../helpers/fixtures.js'
import { PREMIERE_PLAYLIST } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'

test('an upcoming premiere is neither listed nor downloaded', async ({ page }) => {
  // The fixture playlist holds an upcoming premiere, then a regular video.
  const [, regular] = PREMIERE_PLAYLIST.videos
  const playlist = await addPlaylist(page.request, PREMIERE_PLAYLIST, { downloaded: false })
  const videosUrl = `/api/playlists/${encodeURIComponent(playlist.id)}/videos`

  await expect
    .poll(async () => (await (await page.request.get(videosUrl)).json()).map(({ id, status }) => ({ id, status })), {
      message: 'only the regular video, downloaded',
      timeout: 15_000,
    })
    .toEqual([{ id: regular, status: 'DOWNLOADED' }])

  await page.goto(`/playlists/${encodeURIComponent(playlist.id)}`)
  await expect(page.locator('main ul li')).toHaveCount(1)
  await expect(page.locator('main ul li')).toContainText('Snails at rest')
  await expect(page.locator('main')).not.toContainText('Snails premiere')
})
