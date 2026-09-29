import { test, expect } from '@playwright/test'
import { openAddPlaylistDialog, destinationNotice } from '../helpers/addDialog.js'

test('it should show no notice until a playlist is entered', async ({ page }) => {
  await page.goto('/')
  await openAddPlaylistDialog(page)
  const dialog = page.getByRole('dialog')

  await expect(destinationNotice(dialog)).toHaveCount(0)
  await expect(dialog.getByRole('button', { name: /^Create Playlist/ })).toBeDisabled()
})

test('it should explain a value that is not a playlist', async ({ page }) => {
  await page.goto('/')
  await openAddPlaylistDialog(page)
  const dialog = page.getByRole('dialog')

  await dialog.getByLabel('Playlist ID or URL').fill('https://www.youtube.com/watch?v=abc')

  const notice = destinationNotice(dialog)
  await expect(notice).toHaveText(
    'YouTube URL is missing a "list" query parameter (got "https://www.youtube.com/watch?v=abc")',
  )
  await expect(notice).toHaveClass(/text-destructive/)
  await expect(dialog.getByRole('button', { name: /^Create Playlist/ })).toBeDisabled()
})
