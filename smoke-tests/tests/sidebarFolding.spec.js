import { test, expect } from '../helpers/fixtures.js'
import { sectionHeading } from '../helpers/sidebar.js'

test('a folded sidebar section stays folded after a reload', async ({ page }) => {
  await page.goto('/')

  await sectionHeading(page, 'Playlists').click()
  await expect(sectionHeading(page, 'Playlists')).toHaveAttribute('aria-expanded', 'false')
  await expect(page.getByRole('button', { name: 'Add playlist', exact: true })).toHaveCount(0)

  await page.reload()
  await expect(sectionHeading(page, 'Playlists')).toHaveAttribute('aria-expanded', 'false')
  await expect(page.getByRole('button', { name: 'Add playlist', exact: true })).toHaveCount(0)
  // Each section remembers its own state.
  await expect(sectionHeading(page, 'Channels')).toHaveAttribute('aria-expanded', 'true')
  await expect(page.getByRole('button', { name: 'Add channel', exact: true })).toBeVisible()

  await sectionHeading(page, 'Playlists').click()
  await page.reload()
  await expect(sectionHeading(page, 'Playlists')).toHaveAttribute('aria-expanded', 'true')
  await expect(page.getByRole('button', { name: 'Add playlist', exact: true })).toBeVisible()
})

test('a section folded in the mobile drawer stays folded after a reload', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  await page.goto('/')

  // Scoped below the app root, as in mobile-sidebar.spec.js.
  const overlay = page.locator('#root div[aria-hidden="true"]')
  await page.getByRole('button', { name: 'Open menu' }).click()
  await expect(overlay).toBeVisible()
  await sectionHeading(page, 'Channels').click()
  // Folding happens inside the drawer, which stays open.
  await expect(overlay).toBeVisible()
  await expect(sectionHeading(page, 'Channels')).toHaveAttribute('aria-expanded', 'false')
  await expect(page.getByRole('button', { name: 'Add channel', exact: true })).toHaveCount(0)

  await page.reload()
  await page.getByRole('button', { name: 'Open menu' }).click()
  await expect(sectionHeading(page, 'Channels')).toHaveAttribute('aria-expanded', 'false')
  await expect(page.getByRole('button', { name: 'Add channel', exact: true })).toHaveCount(0)

  await sectionHeading(page, 'Channels').click()
  await expect(page.getByRole('button', { name: 'Add channel', exact: true })).toBeVisible()
})
