import { test, expect } from '@playwright/test'
import { openAddChannelDialog, destinationNotice } from '../helpers/addDialog.js'

test('it should show no notice until a handle is entered', async ({ page }) => {
  await page.goto('/')
  await openAddChannelDialog(page)
  const dialog = page.getByRole('dialog')

  await expect(destinationNotice(dialog)).toHaveCount(0)

  await dialog.getByLabel('Channel Handle or URL').fill('@some-handle')
  await expect(destinationNotice(dialog)).toBeVisible()
})
