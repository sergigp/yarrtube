import { test, expect } from '../helpers/fixtures.js'
import { PLAYLIST } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'

test('a playlist page lays out Sync and its YouTube link as icons on desktop, every other action behind a ⋮', async ({ page }) => {
  const playlist = await addPlaylist(page.request, PLAYLIST, { downloaded: false })
  await page.goto(`/playlists/${encodeURIComponent(playlist.id)}`)
  const main = page.locator('main')

  await expect(main.getByRole('button', { name: 'Sync', exact: true })).toBeVisible()
  await expect(main.getByRole('link', { name: 'Open playlist on YouTube', exact: true })).toBeVisible()
  for (const name of ['Mark all watched', 'Exclude from home', 'Hide unwatched count', 'Edit settings', 'Delete']) {
    await expect(main.getByRole('button', { name, exact: true })).toHaveCount(0)
  }
  await main.getByRole('button', { name: /^More actions for / }).click()
  await expect(page.getByRole('menuitem')).toHaveText([
    'Mark all watched',
    'Exclude from home',
    'Hide unwatched count',
    'Edit settings',
    'Clear AI categories',
    'Delete',
  ])
})
