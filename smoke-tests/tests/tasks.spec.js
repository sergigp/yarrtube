import { test, expect } from '@playwright/test'

test('tasks view lists the recurring yt-dlp self-update task', async ({ page }) => {
  await page.goto('/')

  // The daemon seeds this task at startup and it only comes due an hour
  // later, so it is always there, still pending, however the other tests ran.
  await page.getByRole('link', { name: 'Tasks', exact: true }).click()
  const updateYtdlpRow = page.locator('main li', { hasText: 'Updating yt-dlp' })
  await expect(updateYtdlpRow).toBeVisible()
  await expect(updateYtdlpRow.getByText('pending', { exact: true })).toBeVisible()
})
