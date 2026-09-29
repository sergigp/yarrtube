import { test, expect } from '@playwright/test'
import { openAddPlaylistDialog, destinationNotice } from '../helpers/addDialog.js'

test('it should show no notice until a playlist is entered', async ({ page }) => {
  await page.goto('/')
  await openAddPlaylistDialog(page)
  const dialog = page.getByRole('dialog')

  await expect(destinationNotice(dialog)).toHaveCount(0)
  await expect(dialog.getByRole('button', { name: /^Create Playlist/ })).toBeDisabled()
})
