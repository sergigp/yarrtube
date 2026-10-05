import { test, expect } from '@playwright/test'

test('shows the running version in the header', async ({ page }) => {
  await page.goto('/')

  // The smoke image is built without YARRTUBE_VERSION, so it reports `dev`.
  await expect(page.getByRole('banner').getByText('dev', { exact: true })).toBeVisible()
})
