import { test, expect } from '@playwright/test'
import { openAddPlaylistDialog, destinationNotice, slugOf } from '../helpers/addDialog.js'

const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
const PLAYLIST_TITLE = process.env.SMOKE_PLAYLIST_NAME ?? 'test'

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

test('it should state the title, count and destination', async ({ page }) => {
  test.skip(!PLAYLIST_ID, 'SMOKE_PLAYLIST_ID is not set')

  await page.goto('/')
  await openAddPlaylistDialog(page)
  const dialog = page.getByRole('dialog')
  await expect(dialog.getByLabel('Name', { exact: true })).toHaveCount(0)

  await dialog.getByLabel('Playlist ID or URL').fill(PLAYLIST_ID)

  const notice = destinationNotice(dialog)
  await expect(notice).toContainText(
    new RegExp(`^(All \\d+ videos|The only video|Videos) from “${PLAYLIST_TITLE}” will be downloaded to`),
  )
  // Absolute: the videos root, the default `playlists` parent and the slug.
  await expect(notice.locator('code')).toHaveText(
    new RegExp(`^/.+/playlists/${slugOf(PLAYLIST_TITLE)}$`),
  )
  await expect(dialog.getByRole('button', { name: /Advanced options/ })).toHaveAttribute(
    'aria-expanded',
    'false',
  )
})

test('it should expand advanced options from the change action', async ({ page }) => {
  test.skip(!PLAYLIST_ID, 'SMOKE_PLAYLIST_ID is not set')

  await page.goto('/')
  await openAddPlaylistDialog(page)
  const dialog = page.getByRole('dialog')
  await dialog.getByLabel('Playlist ID or URL').fill(PLAYLIST_ID)

  await destinationNotice(dialog).getByRole('button', { name: 'change' }).click()

  await expect(dialog.getByRole('button', { name: /Advanced options/ })).toHaveAttribute(
    'aria-expanded',
    'true',
  )
  await expect(dialog.getByLabel('Folder name')).toBeVisible()
  await expect(dialog.getByLabel('Folder name')).toHaveValue(slugOf(PLAYLIST_TITLE))
})
