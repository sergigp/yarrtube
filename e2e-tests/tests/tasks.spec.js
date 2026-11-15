import { test, expect } from '../helpers/fixtures.js'

test('tasks view lists the recurring yt-dlp self-update task', async ({ page }) => {
  await page.goto('/')

  // Tasks is reached only through the settings menu; the header offers no
  // direct link to it, and adding lives in the sidebar, not the header.
  const header = page.getByRole('banner')
  await expect(header.getByRole('link', { name: 'Tasks', exact: true })).toHaveCount(0)
  await expect(header.getByRole('button', { name: 'Add', exact: true })).toHaveCount(0)

  // The daemon seeds this task at startup and it only comes due an hour
  // later, so it is always there, still scheduled, however the other tests ran.
  await header.getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('menuitem', { name: 'Tasks', exact: true }).click()

  // The view opens on All; the self-update is upkeep, under "Plex & upkeep".
  await expect(page.getByRole('button', { name: /^All/ })).toHaveAttribute('aria-pressed', 'true')
  await page.getByRole('button', { name: /^Plex & upkeep/ }).click()

  const updateYtdlpRow = page.locator('main li', { hasText: 'Update yt-dlp' })
  await expect(updateYtdlpRow).toBeVisible()
  await expect(updateYtdlpRow.getByRole('button', { name: 'Run now' })).toBeVisible()
  // Its state shows by its section, not by a badge.
  await expect(page.locator('main').getByText(/^(pending|queued|running)$/)).toHaveCount(0)
})
