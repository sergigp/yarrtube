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

function playlistPage(seeded) {
  return `/playlists/${encodeURIComponent(seeded.id)}?video=${encodeURIComponent(seeded.video.id)}`
}

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

test('player sticks under the header while the list scrolls @mobile', async ({ page }) => {
  await page.goto(playlistPage(seeded))
  // Expanded details make the page tall enough to scroll on a phone.
  await page.getByRole('button', { name: 'More', exact: true }).tap()
  await expect(page.getByRole('link', { name: 'Open on YouTube' })).toBeVisible()

  // On a phone the document scrolls, not `main`.
  await page.evaluate(() => window.scrollBy(0, 400))
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(0)

  const header = await page.getByRole('banner').boundingBox()
  await expect
    .poll(async () => Math.round((await page.locator('video').boundingBox()).y))
    .toBe(Math.round(header.y + header.height))
})

test('video details start collapsed and More expands them @mobile', async ({ page }) => {
  await page.goto(playlistPage(seeded))
  await expect(page.locator('main h3').first()).toHaveText(seeded.video.title)

  const more = page.getByRole('button', { name: 'More', exact: true })
  await expect(more).toHaveAttribute('aria-expanded', 'false')
  await expect(page.getByRole('link', { name: 'Open on YouTube' })).toHaveCount(0)

  await more.tap()

  await expect(page.getByRole('button', { name: 'Less', exact: true })).toHaveAttribute('aria-expanded', 'true')
  await expect(page.getByRole('link', { name: 'Open on YouTube' })).toBeVisible()
})
