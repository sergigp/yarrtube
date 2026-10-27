import { test, expect } from '../helpers/fixtures.js'
import { PLAYLIST } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'

test('speed menu sets playbackRate to 1.5', async ({ page }) => {
  const playlist = await addPlaylist(page.request, PLAYLIST)
  const [video] = playlist.videos
  await page.goto(`/playlists/${encodeURIComponent(playlist.id)}?video=${encodeURIComponent(video.id)}`)
  const speed = page.getByRole('button', { name: 'Playback speed' })
  await expect(speed).toBeEnabled()

  await speed.click()
  await page.getByRole('menuitemradio', { name: '1.5x' }).click()

  await expect.poll(() => page.locator('video').evaluate((el) => el.playbackRate)).toBe(1.5)
  await expect(speed).toHaveText('1.5x')
})
