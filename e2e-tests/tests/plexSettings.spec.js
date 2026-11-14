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

  await expect(page.getByRole('group', { name: 'Server' })).toBeVisible()
  const watchState = page.getByRole('group', { name: 'Watched status' })
  const labels = page.getByRole('group', { name: 'Categories to Plex' })
  await expect(labels).toBeVisible()
  await expect(page.getByRole('group', { name: 'Category playlists' })).toBeVisible()

  // Every sync fails against the stub, so each row offers a retry.
  await expect(labels.getByRole('button', { name: 'Retry now' })).toBeVisible({ timeout: 15_000 })
  await watchState.getByRole('button', { name: /^(Sync|Retry) now$/ }).click()

  // The page refreshes the status every 5 s, so the finished run can take
  // that long to show.
  await expect(watchState.getByRole('button', { name: 'Retry now' })).toBeEnabled({
    timeout: 15_000,
  })
  await expect(watchState).toContainText('Failed')
})
