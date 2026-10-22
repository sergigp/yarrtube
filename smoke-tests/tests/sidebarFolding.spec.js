import { test, expect } from '../helpers/fixtures.js'
import { expectDrawerOpen, openDrawer, sectionHeading } from '../helpers/sidebar.js'

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

test('a section folded in the mobile drawer stays folded after a reload @mobile', async ({ page }) => {
  await page.goto('/')

  await openDrawer(page)
  await sectionHeading(page, 'Channels').tap()
  // Folding happens inside the drawer, which stays open.
  await expectDrawerOpen(page)
  await expect(sectionHeading(page, 'Channels')).toHaveAttribute('aria-expanded', 'false')
  await expect(page.getByRole('button', { name: 'Add channel', exact: true })).toHaveCount(0)

  await page.reload()
  await openDrawer(page)
  await expect(sectionHeading(page, 'Channels')).toHaveAttribute('aria-expanded', 'false')
  await expect(page.getByRole('button', { name: 'Add channel', exact: true })).toHaveCount(0)

  await sectionHeading(page, 'Channels').tap()
  await expect(page.getByRole('button', { name: 'Add channel', exact: true })).toBeVisible()
})
