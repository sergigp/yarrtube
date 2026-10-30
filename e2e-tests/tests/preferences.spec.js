import { test, expect } from '../helpers/fixtures.js'
import { openAddChannelDialog } from '../helpers/addDialog.js'
import { sectionHeading } from '../helpers/sidebar.js'

test('preferences: turning categories off hides them everywhere, and back on shows them again', async ({ page }) => {
  await page.goto('/')
  const header = page.getByRole('banner')
  const enableCategories = page.getByRole('switch', { name: 'Enable categories' })

  await test.step('the settings menu opens the preferences view', async () => {
    await header.getByRole('button', { name: 'Settings' }).click()
    await expect(page.getByRole('menuitem', { name: 'Report a bug' })).toHaveAttribute(
      'href',
      'https://github.com/sergigp/yarrtube/issues/new',
    )
    await page.getByRole('menuitem', { name: 'Preferences', exact: true }).click()
    await expect(page).toHaveURL(/\/preferences$/)
    await expect(enableCategories).toBeChecked()
  })

  await test.step('categories off: no sidebar section, no dialog field, and it sticks', async () => {
    await enableCategories.click()
    await expect(enableCategories).not.toBeChecked()
    await expect(page.getByRole('switch', { name: 'Show categories' })).toBeDisabled()
    await expect(sectionHeading(page, 'Categories')).toHaveCount(0)

    await openAddChannelDialog(page)
    const dialog = page.getByRole('dialog')
    await expect(dialog.getByRole('group', { name: 'Categories' })).toHaveCount(0)
    await page.keyboard.press('Escape')
    await dialog.waitFor({ state: 'hidden' })

    await page.reload()
    await expect(enableCategories).not.toBeChecked()
    await expect(sectionHeading(page, 'Categories')).toHaveCount(0)
  })

  await test.step('categories back on: the sidebar section returns', async () => {
    await enableCategories.click()
    await expect(enableCategories).toBeChecked()
    await expect(sectionHeading(page, 'Categories')).toBeVisible()
  })
})
