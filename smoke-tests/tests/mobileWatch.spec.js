import { test, expect } from '../helpers/fixtures.js'
import { deleteSeeded, seedDownloadedPlaylist } from '../helpers/seed.js'
import { assertVideoPlays } from '../helpers/video.js'

test.use({ keepLibrary: true })

let seeded
test.beforeAll(async ({ request }) => {
  seeded = await seedDownloadedPlaylist(request)
})

test.afterAll(async ({ request }) => {
  await deleteSeeded(request, seeded)
})

test('home card opens the video selected, plays and resumes after reload @mobile', async ({ page }) => {
  const { title } = seeded.video
  await page.goto('/')

  await page.locator('main').getByRole('link', { name: title, exact: true }).first().tap()

  await expect(page).toHaveURL(new RegExp(`\\?video=${seeded.video.id}$`))
  await expect(page.locator('main h3').first()).toHaveText(title)
  await assertVideoPlays(page)

  // Pausing halfway through records the position, so a reload resumes there.
  const video = page.locator('video')
  const progressReported = page.waitForResponse(
    (response) => response.url().endsWith('/progress') && response.request().method() === 'POST',
  )
  await video.evaluate((el) => {
    el.currentTime = el.duration / 2
    el.pause()
  })
  expect((await progressReported).status()).toBe(200)
  await page.reload()
  await expect.poll(() => video.evaluate((el) => el.currentTime), { timeout: 15_000 }).toBeGreaterThan(0)
})

test.fixme('player sticks under the header while the list scrolls @mobile', async () => {})

test.fixme('video details start collapsed and More expands them @mobile', async () => {})
