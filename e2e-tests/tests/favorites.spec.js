import { test, expect } from '../helpers/fixtures.js'
import { PLAYLIST } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'
import { enterWatching } from '../helpers/video.js'

/** Waits for the next `method` response to a URL ending in `suffix`. */
function nextResponse(page, method, suffix) {
  return page.waitForResponse(
    (response) => response.request().method() === method && response.url().endsWith(suffix),
  )
}

test('star a video: it shows under Favorites, and unstarring takes it off', async ({ page }) => {
  const playlist = await addPlaylist(page.request, PLAYLIST)
  await page.goto(`/playlists/${encodeURIComponent(playlist.id)}`)
  await enterWatching(page)
  const main = page.locator('main')
  const title = await main.getByRole('heading', { level: 3 }).first().textContent()

  const favorited = nextResponse(page, 'PUT', '/favorite')
  await main.getByRole('button', { name: 'Add to favorites' }).click()
  expect((await favorited).ok()).toBe(true)
  await expect(main.getByRole('button', { name: 'Remove from favorites', pressed: true })).toBeVisible()

  const listed = nextResponse(page, 'GET', '/api/favorites/videos')
  await page.getByRole('link', { name: 'Favorites', exact: true }).click()
  expect((await listed).ok()).toBe(true)
  await expect(main.getByRole('heading', { name: 'Favorites', level: 1 })).toBeVisible()
  const card = main.locator('li:has([data-slot="card-thumbnail"]) a').filter({ hasText: title })
  await expect(card).toHaveCount(1)

  await card.click()
  await expect(main.getByRole('heading', { level: 3 }).first()).toHaveText(title)
  const unfavorited = nextResponse(page, 'DELETE', '/favorite')
  await main.getByRole('button', { name: 'Remove from favorites' }).click()
  expect((await unfavorited).ok()).toBe(true)
  // Unstarred, it leaves the favorites it was playing from.
  await expect(main.getByText('No video selected.')).toBeVisible()

  await page.goto('/favorites')
  await expect(main.getByRole('heading', { name: 'No favorites yet' })).toBeVisible()
})
