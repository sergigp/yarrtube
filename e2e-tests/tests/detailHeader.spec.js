import { test, expect } from '../helpers/fixtures.js'
import { PLAYLIST } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'

test("a playlist page lays every header action out as a button on desktop, with no ⋮", async ({ page }) => {
  const playlist = await addPlaylist(page.request, PLAYLIST, { downloaded: false })
  await page.goto(`/playlists/${encodeURIComponent(playlist.id)}`)
  const main = page.locator('main')

  for (const name of [
    'Sync',
    'Mark all watched',
    'Exclude from home',
    'Hide unwatched count',
    'Edit settings',
    'Delete',
  ]) {
    await expect(main.getByRole('button', { name, exact: true })).toBeVisible()
  }
  await expect(page.getByRole('button', { name: /^More actions for / })).toHaveCount(0)
})
