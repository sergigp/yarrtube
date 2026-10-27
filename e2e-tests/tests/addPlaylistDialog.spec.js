import { test, expect } from '../helpers/fixtures.js'
import {
  openAddPlaylistDialog,
  openAdvancedOptions,
  destinationNotice,
  fillPlaylist,
  slugOf,
} from '../helpers/addDialog.js'
import { PLAYLIST } from '../helpers/fakeYoutube.js'

const PLAYLIST_ID = PLAYLIST.id
const PLAYLIST_TITLE = PLAYLIST.title

test('it should show no notice until a playlist is entered', async ({ page }) => {
  await page.goto('/')
  await openAddPlaylistDialog(page)
  const dialog = page.getByRole('dialog')

  await expect(destinationNotice(dialog)).toHaveCount(0)
  await expect(dialog.getByRole('button', { name: /^Create Playlist/ })).toBeDisabled()
})

test.describe(() => {
  // The lookup rejects the value, which the notice explains.
  test.use({ expectedApiErrors: [{ method: 'GET', path: '/api/playlists/preview', status: 400 }] })

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
})

test('it should state the title, count and destination', async ({ page }) => {
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

test('it should auto-fill the folder name under advanced options', async ({ page }) => {
  await page.goto('/')
  await openAddPlaylistDialog(page)
  const dialog = await fillPlaylist(page, { url: PLAYLIST_ID })

  await openAdvancedOptions(dialog)

  await expect(dialog.getByLabel('Folder name')).toBeVisible()
  await expect(dialog.getByLabel('Folder name')).toHaveValue(slugOf(PLAYLIST_TITLE))
})

test.describe(() => {
  // The lookup misses, which the notice reports.
  test.use({ expectedApiErrors: [{ method: 'GET', path: '/api/playlists/preview', status: 404 }] })

  test('unknown playlist id is reported in the Add playlist dialog', async ({ page }) => {
    await page.goto('/')
    await openAddPlaylistDialog(page)
    const dialog = page.getByRole('dialog')

    await dialog.getByLabel('Playlist ID or URL').fill(`PLyarrtubeMissing${Date.now()}`)

    const notice = destinationNotice(dialog)
    await expect(notice).toContainText('does not exist', { timeout: 15_000 })
    await expect(notice).toHaveClass(/text-destructive/)
    await expect(dialog.getByRole('button', { name: /^Create Playlist/ })).toBeDisabled()
  })
})
