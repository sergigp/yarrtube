import { test, expect } from '@playwright/test'

test('mobile sidebar opens via the menu button and closes', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  await page.goto('/')

  const overlay = page.locator('div[aria-hidden="true"]')
  await expect(overlay).toHaveCount(0)

  await page.getByRole('button', { name: 'Open menu' }).click()
  await expect(overlay).toBeVisible()

  await page.getByRole('button', { name: 'Close menu' }).click()
  await expect(overlay).toHaveCount(0)

  await page.getByRole('button', { name: 'Open menu' }).click()
  await expect(overlay).toBeVisible()

  await overlay.click()
  await expect(overlay).toHaveCount(0)
})
