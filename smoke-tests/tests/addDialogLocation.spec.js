import { test, expect } from '@playwright/test'
import {
  openAddChannelDialog,
  openAddPlaylistDialog,
  openAdvancedOptions,
  openFolderBrowser,
  browseToParent,
  breadcrumbLinks,
  folderEntry,
  destinationNotice,
  fillChannel,
  fillPlaylist,
  slugOf,
  submitPlaylist,
} from '../helpers/addDialog.js'
import { waitForVideoStatus } from '../helpers/video.js'
import { deleteItem } from '../helpers/sidebar.js'

const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
const PLAYLIST_NAME = process.env.SMOKE_PLAYLIST_NAME ?? 'test'

/** The immediate subdirectory names the daemon reports for `path`. */
async function listDirectories(page, path) {
  const query = path ? `?path=${encodeURIComponent(path)}` : ''
  const response = await page.request.get(`/api/directories${query}`)
  expect(response.ok()).toBeTruthy()
  return (await response.json()).entries.map((entry) => entry.name)
}

function escapeForRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/** An absolute destination ending in `path`, as the notices state it. */
function endingIn(path) {
  return new RegExp(`^/.+/${escapeForRegExp(path)}$`)
}

test('it should create a playlist into a browsed parent folder', async ({ page }) => {
  test.skip(!PLAYLIST_ID, 'SMOKE_PLAYLIST_ID is not set')
  const folderName = 'browsed-parent-target'

  await page.goto('/')
  await openAddPlaylistDialog(page)
  const dialog = await fillPlaylist(page, {
    url: PLAYLIST_ID,
    parent: 'playlists',
    folderName,
  })

  // The notice names the destination the videos are about to land in.
  await expect(destinationNotice(dialog).locator('code')).toHaveText(
    endingIn(`playlists/${folderName}`),
  )
  await dialog.getByRole('button', { name: /^Create Playlist/ }).click()
  await dialog.waitFor({ state: 'hidden' })

  const sidebarLink = page
    .locator('h3:text-is("Playlists") ~ ul')
    .getByRole('link', { name: PLAYLIST_NAME })
  await expect(sidebarLink).toBeVisible()
  await sidebarLink.click()
  await waitForVideoStatus(page, { status: 'DOWNLOADED', timeoutMs: 240_000 })

  // The videos went to the stated destination, not anywhere else.
  expect(await listDirectories(page, 'playlists')).toContain(folderName)

  await deleteItem(page, { section: 'Playlists', name: PLAYLIST_NAME })
})

test('it should create a playlist into a staged parent folder that did not exist', async ({
  page,
}) => {
  test.skip(!PLAYLIST_ID, 'SMOKE_PLAYLIST_ID is not set')
  const stagedParent = `staged-${Date.now()}`
  const folderName = 'staged-parent-target'

  await page.goto('/')
  // The staged parent must genuinely not exist, or this proves nothing.
  expect(await listDirectories(page, '')).not.toContain(stagedParent)

  await openAddPlaylistDialog(page)
  const dialog = await fillPlaylist(page, {
    url: PLAYLIST_ID,
    parent: stagedParent,
    folderName,
  })

  await expect(destinationNotice(dialog).locator('code')).toHaveText(
    endingIn(`${stagedParent}/${folderName}`),
  )
  // Staging writes nothing: the parent is still absent from the videos root.
  expect(await listDirectories(page, '')).not.toContain(stagedParent)

  await dialog.getByRole('button', { name: /^Create Playlist/ }).click()
  await dialog.waitFor({ state: 'hidden' })

  const sidebarLink = page
    .locator('h3:text-is("Playlists") ~ ul')
    .getByRole('link', { name: PLAYLIST_NAME })
  await sidebarLink.click()
  await waitForVideoStatus(page, { status: 'DOWNLOADED', timeoutMs: 240_000 })

  // Both directories exist once the first download has run.
  expect(await listDirectories(page, '')).toContain(stagedParent)
  expect(await listDirectories(page, stagedParent)).toContain(folderName)

  await deleteItem(page, { section: 'Playlists', name: PLAYLIST_NAME })
})

test('it should reach a sibling of the default parent via the breadcrumb', async ({ page }) => {
  await page.goto('/')
  await openAddChannelDialog(page)
  const dialog = await fillChannel(page, { handle: '@some-handle' })
  await openAdvancedOptions(dialog)
  await openFolderBrowser(dialog)

  // The browser opens on the default parent, `channels`.
  await expect(dialog.getByRole('navigation', { name: 'Folder path' })).toContainText('channels')

  // One click on the videos root, then into its sibling — without ever
  // leaving the dialog or typing a path.
  await breadcrumbLinks(dialog).first().click()
  await folderEntry(dialog, 'playlists').click()

  await expect(destinationNotice(dialog).locator('code')).toHaveText(
    endingIn('playlists/some-handle'),
  )
  await expect(dialog.getByRole('navigation', { name: 'Folder path' })).toContainText('playlists')
})

test('it should descend into an existing directory when the create-folder step names one', async ({
  page,
}) => {
  await page.goto('/')
  await openAddChannelDialog(page)
  const dialog = await fillChannel(page, { handle: '@some-handle' })
  await openAdvancedOptions(dialog)
  await openFolderBrowser(dialog)
  await breadcrumbLinks(dialog).first().click()

  // `playlists` is seeded at daemon startup, so naming it here names
  // something that already exists.
  await dialog.getByRole('button', { name: 'New folder' }).click()
  await dialog.getByLabel('New folder name').fill('playlists')
  await dialog.getByRole('button', { name: 'Use folder' }).click()

  await dialog.getByLabel('Folder name').fill('descended-target')
  await expect(destinationNotice(dialog).locator('code')).toHaveText(
    endingIn('playlists/descended-target'),
  )
  // Descended into, not adopted as new: the browser lists it rather than
  // reporting it as not existing yet.
  await expect(dialog.getByText(/Does not exist yet/)).toHaveCount(0)
})

test('it should block submission when the destination is already used by another playlist', async ({
  page,
}) => {
  test.skip(!PLAYLIST_ID, 'SMOKE_PLAYLIST_ID is not set')

  await page.goto('/')
  await openAddPlaylistDialog(page)
  await submitPlaylist(page, { url: PLAYLIST_ID })
  const sidebarLink = page
    .locator('h3:text-is("Playlists") ~ ul')
    .getByRole('link', { name: PLAYLIST_NAME })
  await expect(sidebarLink).toBeVisible()

  // Both dialogs share the location logic, so the channel dialog pointed at
  // the playlist's folder runs into the same conflict.
  await openAddChannelDialog(page)
  const dialog = await fillChannel(page, {
    handle: '@some-handle',
    parent: 'playlists',
    folderName: slugOf(PLAYLIST_NAME),
  })

  const notice = destinationNotice(dialog)
  await expect(notice).toContainText(`is already used by ${PLAYLIST_NAME}`)
  await expect(notice.locator('code')).toHaveText(endingIn(`playlists/${slugOf(PLAYLIST_NAME)}`))
  await expect(dialog.getByRole('button', { name: /^Create Channel/ })).toBeDisabled()
  await dialog.getByRole('button', { name: 'Close' }).click()

  await deleteItem(page, { section: 'Playlists', name: PLAYLIST_NAME })
})

test('it should reject a folder name containing a slash', async ({ page }) => {
  await page.goto('/')
  await openAddPlaylistDialog(page)
  const dialog = page.getByRole('dialog')
  await openAdvancedOptions(dialog)

  await dialog.getByLabel('Folder name').fill('kids/movies')

  await expect(dialog.getByText(/Folder name must not contain/)).toBeVisible()
  await expect(dialog.getByRole('button', { name: /^Create Playlist/ })).toBeDisabled()
})
