import { test, expect } from '../helpers/fixtures.js'
import { addPlaylist } from '../helpers/seed.js'
import { LONG_PLAYLIST, PLAYLIST } from '../helpers/fakeYoutube.js'
import { assertVideoPlays } from '../helpers/video.js'

let seeded
test.beforeEach(async ({ request }) => {
  const playlist = await addPlaylist(request, PLAYLIST)
  seeded = { ...playlist, video: playlist.videos[0] }
})

/** Waits until `video` is playing on its own: not paused, and advancing. */
async function expectPlaying(video) {
  await expect
    .poll(() => video.evaluate((el) => !el.paused && el.currentTime > 0), {
      message: 'video playing',
      timeout: 15_000,
    })
    .toBe(true)
}

function playlistPage(seeded) {
  return `/playlists/${encodeURIComponent(seeded.id)}?video=${encodeURIComponent(seeded.video.id)}`
}

test('home card opens the video selected, plays and resumes after reload @mobile', async ({ page }) => {
  const { title } = seeded.video
  await page.goto('/')

  await page.locator('main').getByRole('link', { name: title, exact: true }).first().tap()

  await expect(page).toHaveURL(new RegExp(`\\?video=${seeded.video.id}$`))
  await expect(page.locator('main h3').first()).toHaveText(title)
  // It starts without a tap on play.
  await expectPlaying(page.locator('video'))
  await assertVideoPlays(page)

  // Pausing at 40 s (past the 30 s that count as started) records the
  // position, so a reload resumes there.
  const video = page.locator('video')
  const progressReported = page.waitForResponse(
    (response) => response.url().endsWith('/progress') && response.request().method() === 'POST',
  )
  await video.evaluate((el) => {
    el.currentTime = 40
    el.pause()
  })
  expect((await progressReported).status()).toBe(200)
  await page.reload()
  await expect.poll(() => video.evaluate((el) => el.currentTime), { timeout: 15_000 }).toBeGreaterThan(0)
})

test('player sticks under the header while the list scrolls @mobile', async ({ page }) => {
  const playlist = await addPlaylist(page.request, LONG_PLAYLIST)
  await page.goto(
    `/playlists/${encodeURIComponent(playlist.id)}?video=${encodeURIComponent(playlist.videos[0].id)}`,
  )
  await expect(page.locator('main ul li')).toHaveCount(LONG_PLAYLIST.videos.length)

  // On a phone the document scrolls, not `main`.
  await page.evaluate(() => window.scrollBy(0, 400))
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(0)

  const header = await page.getByRole('banner').boundingBox()
  await expect
    .poll(async () => Math.round((await page.locator('video').boundingBox()).y))
    .toBe(Math.round(header.y + header.height))
})

test('video details show without a More control @mobile', async ({ page }) => {
  await page.goto(playlistPage(seeded))
  await expect(page.locator('main h3').first()).toHaveText(seeded.video.title)

  await expect(page.getByRole('link', { name: 'Open video on YouTube' })).toBeVisible()
  await expect(page.locator('main').getByText(seeded.path, { exact: false })).toBeVisible()
  await expect(page.getByRole('button', { name: 'More', exact: true })).toHaveCount(0)
})

test('picking a video in the queue starts it playing @mobile', async ({ page }) => {
  const playlist = await addPlaylist(page.request, LONG_PLAYLIST)
  const [first, second] = playlist.videos
  await page.goto(`/playlists/${encodeURIComponent(playlist.id)}?video=${encodeURIComponent(first.id)}`)
  const video = page.locator('video')
  await expect(video).toBeVisible()
  await video.evaluate((el) => {
    el.dataset.e2eElement = 'first'
    el.pause()
  })
  const firstSrc = await video.getAttribute('src')

  await page
    .locator('main')
    .getByRole('complementary', { name: 'Queue' })
    .getByRole('button')
    .filter({ hasText: second.title })
    .tap()

  await expect(page.locator('main h3').first()).toHaveText(second.title)
  await expect(video).not.toHaveAttribute('src', firstSrc)
  await expectPlaying(video)
  // The same element plays it, which is what lets iOS start it from the tap.
  await expect(video).toHaveAttribute('data-e2e-element', 'first')
})

test('rotating to landscape fills the screen with the video and back restores the page @mobile', async ({
  page,
}) => {
  const playlist = await addPlaylist(page.request, LONG_PLAYLIST)
  await page.goto(
    `/playlists/${encodeURIComponent(playlist.id)}?video=${encodeURIComponent(playlist.videos[0].id)}`,
  )
  await expect(page.locator('main ul li')).toHaveCount(LONG_PLAYLIST.videos.length)
  const video = page.locator('video')
  await expectPlaying(video)
  await video.evaluate((el) => {
    el.dataset.e2eElement = 'portrait'
  })
  // Up to the scroll event, not just `scrollY`: it fires a frame later, and
  // the page only knows where to scroll back from it. Turning the phone in
  // that frame is not something a person does.
  const scrollY = await page.evaluate(
    () =>
      new Promise((resolve) => {
        window.addEventListener('scroll', () => resolve(window.scrollY), { once: true })
        window.scrollTo(0, 300)
      }),
  )
  expect(scrollY).toBeGreaterThan(0)
  const portrait = page.viewportSize()

  await page.setViewportSize({ width: portrait.height + 180, height: portrait.width })

  const landscape = page.viewportSize()
  await expect
    .poll(async () => {
      const box = await video.boundingBox()
      return [box.x, box.y, box.width, box.height].map(Math.round)
    })
    .toEqual([0, 0, landscape.width, landscape.height])
  // Only the video shows: the header's corner is covered by the player.
  const topLeftIsPlayer = await page.evaluate(
    () => document.elementFromPoint(5, 5)?.closest('[data-landscape-takeover]') !== null,
  )
  expect(topLeftIsPlayer).toBe(true)
  await expect(page.getByRole('navigation', { name: 'Chapters' })).toHaveCount(0)
  expect(await video.evaluate((el) => el.paused)).toBe(false)

  await page.setViewportSize(portrait)

  await expect(page.locator('[data-landscape-takeover]')).toHaveCount(0)
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBe(scrollY)
  await expect(video).toHaveAttribute('data-e2e-element', 'portrait')
  expect(await video.evaluate((el) => el.paused)).toBe(false)
})
