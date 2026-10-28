import { test, expect } from '../helpers/fixtures.js'
import { PLAYLIST } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'
import { assertVideoPlays } from '../helpers/video.js'

test('← / → seek the playing video 10 s, with on-screen feedback', async ({ page }) => {
  const playlist = await addPlaylist(page.request, PLAYLIST)
  const [video] = playlist.videos
  await page.goto(`/playlists/${encodeURIComponent(playlist.id)}?video=${encodeURIComponent(video.id)}`)
  await assertVideoPlays(page)
  const player = page.locator('video')
  // Paused, so the seeks read back exactly.
  await player.evaluate((el) => {
    el.pause()
    el.currentTime = 30
  })
  const currentTime = () => player.evaluate((el) => el.currentTime)
  const feedback = page.getByRole('status').filter({ hasText: / seconds$/ })

  await test.step('→ seeks forward, and presses in a row add up', async () => {
    await page.keyboard.press('ArrowRight')
    await expect(feedback).toHaveText('10 seconds')
    await page.keyboard.press('ArrowRight')
    await expect(feedback).toHaveText('20 seconds')
    await expect.poll(currentTime).toBe(50)
  })

  await test.step('the feedback fades away on its own', async () => {
    await expect(feedback).toHaveCount(0)
  })

  await test.step('← seeks back', async () => {
    await page.keyboard.press('ArrowLeft')
    await expect(feedback).toHaveText('10 seconds')
    await expect.poll(currentTime).toBe(40)
  })

  await test.step('arrows typed into the sidebar search are left alone', async () => {
    await expect(feedback).toHaveCount(0)
    // The sidebar shows its search only for big libraries; any text field will do.
    await page.evaluate(() => {
      const input = document.createElement('input')
      input.setAttribute('aria-label', 'Scratch field')
      document.body.append(input)
    })
    await page.getByRole('textbox', { name: 'Scratch field' }).focus()
    await page.keyboard.press('ArrowLeft')
    await expect(feedback).toHaveCount(0)
    expect(await currentTime()).toBe(40)
  })
})
