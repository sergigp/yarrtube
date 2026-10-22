import { test, expect } from '../helpers/fixtures.js'
import { deleteSeeded, seedDownloadedPlaylist } from '../helpers/seed.js'

test.use({ keepLibrary: true })

let seeded
test.beforeAll(async ({ request }) => {
  seeded = await seedDownloadedPlaylist(request)
})

test.afterAll(async ({ request }) => {
  await deleteSeeded(request, seeded)
})

test('speed menu sets playbackRate to 1.5', async ({ page }) => {
  await page.goto(`/playlists/${encodeURIComponent(seeded.id)}?video=${encodeURIComponent(seeded.video.id)}`)
  const speed = page.getByRole('button', { name: 'Playback speed' })
  await expect(speed).toBeEnabled()

  await speed.click()
  await page.getByRole('menuitemradio', { name: '1.5x' }).click()

  await expect.poll(() => page.locator('video').evaluate((el) => el.playbackRate)).toBe(1.5)
  await expect(speed).toHaveText('1.5x')
})
