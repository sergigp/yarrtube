import { test, expect } from '../helpers/fixtures.js'

// The daemon runs against a stub Plex that only answers library scans, so
// every sync reaches it and then fails: enough to show the status the syncs
// record and to run one now.
test('Plex settings: show how the syncs are doing and run one now', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('banner').getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('menuitem', { name: 'Settings', exact: true }).click()
  await page
    .getByRole('navigation', { name: 'Settings sections' })
    .getByRole('link', { name: 'Plex', exact: true })
    .click()
  await expect(page).toHaveURL(/\/settings\/plex$/)

  await expect(page.getByRole('group', { name: 'Connection' })).toBeVisible()
  const watchState = page.getByRole('group', { name: 'Watch state' })
  await expect(page.getByRole('group', { name: 'Labels & playlists' })).toBeVisible()
  await expect(page.getByRole('group', { name: 'Category playlists' })).toBeVisible()

  await watchState.getByRole('button', { name: 'Sync now' }).click()

  await expect(watchState.getByRole('button', { name: 'Sync now' })).toBeEnabled()
  await expect(watchState).toContainText('Last run')
})
