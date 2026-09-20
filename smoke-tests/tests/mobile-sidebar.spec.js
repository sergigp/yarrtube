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

  // Click a point clear of the sidebar itself (up to 288px wide, left-aligned)
  // rather than the overlay's default center, which at this viewport width
  // would fall under the higher-stacked sidebar and could intercept the click.
  await overlay.click({ position: { x: 370, y: 20 } })
  await expect(overlay).toHaveCount(0)
})
