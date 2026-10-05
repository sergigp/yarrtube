import { test, expect } from '@playwright/test'

test('tasks view lists the recurring yt-dlp self-update task', async ({ page }) => {
  await page.goto('/')

  // Tasks is reached only through the settings menu; the header offers no
  // direct link to it, and adding lives in the sidebar, not the header.
  const header = page.getByRole('banner')
  await expect(header.getByRole('link', { name: 'Tasks', exact: true })).toHaveCount(0)
  await expect(header.getByRole('button', { name: 'Add', exact: true })).toHaveCount(0)

  // The daemon seeds this task at startup and it only comes due an hour
  // later, so it is always there, still pending, however the other tests ran.
  await header.getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('menuitem', { name: 'Tasks', exact: true }).click()

  // The view opens on the Active tab (running tasks only); the recurring
  // self-update is a pending maintenance task, listed under "Other".
  await page.getByRole('tab', { name: /^Other/ }).click()

  const updateYtdlpRow = page.locator('main li', { hasText: 'Updating yt-dlp' })
  await expect(updateYtdlpRow).toBeVisible()
  await expect(updateYtdlpRow.getByText('pending', { exact: true })).toBeVisible()
})
