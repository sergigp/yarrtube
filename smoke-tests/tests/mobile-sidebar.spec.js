import { test, expect } from '../helpers/fixtures.js'

test('mobile sidebar opens via the menu button and closes', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  await page.goto('/')

  const overlay = page.locator('#root div[aria-hidden="true"]')
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

test('mobile sidebar stays closable when the announcement appears after it opened', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  // Hold the announcements response until the sidebar is open, so the bar
  // appears above the header afterwards and pushes it down.
  let releaseAnnouncements
  const announcementsHeld = new Promise((resolve) => (releaseAnnouncements = resolve))
  await page.route('**/announcements/announcements.json', async (route) => {
    await announcementsHeld
    await route.fulfill({
      json: [{ id: 'smoke-late-banner', text: 'A late announcement' }],
    })
  })
  await page.goto('/')

  const overlay = page.locator('#root div[aria-hidden="true"]')
  await page.getByRole('button', { name: 'Open menu' }).click()
  await expect(overlay).toBeVisible()
  releaseAnnouncements()
  await expect(page.getByRole('region', { name: 'Announcement' })).toBeVisible()

  await page.getByRole('button', { name: 'Close menu' }).click({ timeout: 5000 })
  await expect(overlay).toHaveCount(0)
})

test('it should close the drawer and open the add channel dialog', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  await page.goto('/')

  // Scoped below the app root: while the dialog is open, Radix marks the root
  // itself `aria-hidden`, and the dialog's own overlay is portalled to body.
  const overlay = page.locator('#root div[aria-hidden="true"]')
  await page.getByRole('button', { name: 'Open menu' }).click()
  await expect(overlay).toBeVisible()

  await page.getByRole('button', { name: 'Add channel', exact: true }).click()

  await expect(overlay).toHaveCount(0)
  await expect(page.getByRole('dialog', { name: 'Add channel' })).toBeVisible()
})
