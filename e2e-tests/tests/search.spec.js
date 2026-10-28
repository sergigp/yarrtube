import { test, expect } from '../helpers/fixtures.js'
import { SEARCH_PLAYLISTS } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'
import { sectionHeading, sectionRows } from '../helpers/sidebar.js'

// The search boxes only show beyond 15 sources (sidebar) or 15 tasks in a tab
// (Tasks view): the 16 empty fixture playlists, each with its sync, are just
// enough.
test.beforeEach(async ({ request }) => {
  for (const playlist of SEARCH_PLAYLISTS) {
    await addPlaylist(request, playlist, { downloaded: false })
  }
})

test('sidebar search lists only matching playlists', async ({ page }) => {
  await page.goto('/')
  await expect(sectionRows(page, 'Playlists')).toHaveCount(SEARCH_PLAYLISTS.length)

  await page.getByRole('searchbox', { name: 'Search categories, channels and playlists' }).fill('search 07')

  await expect(sectionRows(page, 'Playlists')).toHaveText([/^Search 07/])
})

test('tasks search lists only matching tasks', async ({ page }) => {
  await page.goto('/tasks')
  await page.getByRole('tab', { name: /^Syncs/ }).click()
  const syncs = page.locator('main li', { hasText: /^Syncing playlist / })
  await expect(syncs).toHaveCount(SEARCH_PLAYLISTS.length)

  await page.getByRole('searchbox', { name: 'Search tasks' }).fill('Search 07')

  await expect(syncs).toHaveText([/^Syncing playlist Search 07/])
})

test('sidebar search sits above the categories and filters them too', async ({ request, page }) => {
  const category = await request.post('/api/categories', { data: { name: 'Science' } })
  expect(category.ok(), await category.text()).toBeTruthy()
  await page.goto('/')
  const search = page.getByRole('searchbox', { name: 'Search categories, channels and playlists' })
  await expect(sectionRows(page, 'Categories')).toHaveText([/^Science/])

  const searchBox = await search.boundingBox()
  const categoriesBox = await sectionHeading(page, 'Categories').boundingBox()
  expect(searchBox.y).toBeLessThan(categoriesBox.y)

  await test.step('a category match hides the sections with none', async () => {
    await search.fill('scien')
    await expect(sectionRows(page, 'Categories')).toHaveText([/^Science/])
    await expect(sectionHeading(page, 'Playlists')).toHaveCount(0)
  })

  await test.step('no category match hides the Categories section', async () => {
    await search.fill('search 07')
    await expect(sectionRows(page, 'Playlists')).toHaveText([/^Search 07/])
    await expect(sectionHeading(page, 'Categories')).toHaveCount(0)
  })
})
