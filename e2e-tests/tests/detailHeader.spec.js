import { test, expect } from '../helpers/fixtures.js'
import { PLAYLIST } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'

test("a playlist page lays every header action but mark-all-watched and delete out as a button on desktop, both behind a ⋮", async ({ page }) => {
  const playlist = await addPlaylist(page.request, PLAYLIST, { downloaded: false })
  await page.goto(`/playlists/${encodeURIComponent(playlist.id)}`)
  const main = page.locator('main')

  for (const name of [
    'Sync',
    'Exclude from home',
    'Hide unwatched count',
    'Edit settings',
  ]) {
    await expect(main.getByRole('button', { name, exact: true })).toBeVisible()
  }
  for (const name of ['Mark all watched', 'Delete']) {
    await expect(main.getByRole('button', { name, exact: true })).toHaveCount(0)
  }
  await main.getByRole('button', { name: /^More actions for / }).click()
  await expect(page.getByRole('menuitem')).toHaveText(['Mark all watched', 'Delete'])
})
