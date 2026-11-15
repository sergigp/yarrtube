import { test, expect } from '../helpers/fixtures.js'
import { PLAYLIST, SEARCH_PLAYLISTS } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'

test.describe('with 16 empty playlists', () => {
  // The 16 empty fixture playlists, each with its sync due about the same
  // time, so the Tasks view batches them.
  test.beforeEach(async ({ request }) => {
    for (const playlist of SEARCH_PLAYLISTS) {
      await addPlaylist(request, playlist, { downloaded: false })
    }
  })

  test('header search lists only matching playlists, and opens the chosen one', async ({ page }) => {
    await page.goto('/')
    await page.getByRole('banner').getByRole('button', { name: 'Search' }).click()
    const dialog = page.getByRole('dialog')

    await dialog.getByRole('combobox', { name: 'Search' }).fill('search 07')

    const playlists = dialog.getByRole('group', { name: 'Playlists' }).getByRole('option')
    await expect(playlists).toHaveText([/^Search 07/])
    await playlists.first().click()
    await expect(dialog).toBeHidden()
    await expect(page).toHaveURL(/\/playlists\/PLyarrtubeFixtureSearch07$/)
  })

  test('tasks search lists only matching tasks', async ({ page }) => {
    await page.goto('/tasks')
    await page.getByRole('button', { name: /^Syncs/ }).click()
    // Batched: a sync may still be running its first check, outside the batch.
    await expect(page.locator('main li', { hasText: /^.*Check \d+ channels and playlists for new videos/ })).toHaveCount(1)

    await page.getByRole('searchbox', { name: 'Search tasks' }).fill('Search 07')

    await expect(page.locator('main li', { hasText: /for new videos/ })).toHaveText([
      /Check Search 07 for new videos/,
    ])
  })

})

test('header search finds a downloaded video by title and opens it in its playlist', async ({ page }) => {
  const playlist = await addPlaylist(page.request, PLAYLIST)
  const video = playlist.videos[0]
  await page.goto('/')

  await page.keyboard.press('ControlOrMeta+k')
  const dialog = page.getByRole('dialog')
  await dialog.getByRole('combobox', { name: 'Search' }).fill(video.title.slice(0, 10).toLowerCase())

  const result = dialog.getByRole('group', { name: 'Videos' }).getByRole('option', { name: new RegExp(video.title) })
  await expect(result.first()).toBeVisible()
  await result.first().click()
  await expect(dialog).toBeHidden()
  const url = new URL(page.url())
  expect(`${url.pathname}${url.search}`).toBe(
    `/playlists/${encodeURIComponent(playlist.id)}?video=${encodeURIComponent(video.id)}`,
  )
})
