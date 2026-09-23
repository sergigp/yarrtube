import { test, expect } from '@playwright/test'
import {
  openAddDialog,
  openFolderBrowser,
  browseToParent,
  breadcrumbLinks,
  folderEntry,
  destinationPreview,
  fillPlaylist,
  submitPlaylist,
} from '../helpers/addDialog.js'
import { waitForVideoStatus } from '../helpers/video.js'
import { deleteItem } from '../helpers/sidebar.js'

const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
const PLAYLIST_NAME = process.env.SMOKE_PLAYLIST_NAME ?? 'yarrtube smoke tests'

/** The immediate subdirectory names the daemon reports for `path`. */
async function listDirectories(page, path) {
  const query = path ? `?path=${encodeURIComponent(path)}` : ''
  const response = await page.request.get(`/api/directories${query}`)
  expect(response.ok()).toBeTruthy()
  return (await response.json()).entries.map((entry) => entry.name)
}

test('it should create a playlist into a browsed parent folder', async ({ page }) => {
  test.skip(!PLAYLIST_ID, 'SMOKE_PLAYLIST_ID is not set')
  const folderName = 'browsed-parent-target'

  await page.goto('/')
  await openAddDialog(page)
  const dialog = await fillPlaylist(page, {
    url: PLAYLIST_ID,
    name: PLAYLIST_NAME,
    parent: 'playlists',
    folderName,
  })

  // The preview names the destination the videos are about to land in.
  await expect(destinationPreview(dialog)).toContainText(`playlists/${folderName}`)
  await dialog.getByRole('button', { name: /^Create Playlist/ }).click()
  await dialog.waitFor({ state: 'hidden' })

  const sidebarLink = page
    .locator('h3:text-is("Playlists") ~ ul')
    .getByRole('link', { name: PLAYLIST_NAME })
  await expect(sidebarLink).toBeVisible()
  await sidebarLink.click()
  await waitForVideoStatus(page, { status: 'DOWNLOADED', timeoutMs: 240_000 })

  // The videos went to the previewed destination, not anywhere else.
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

  await openAddDialog(page)
  const dialog = await fillPlaylist(page, {
    url: PLAYLIST_ID,
    name: PLAYLIST_NAME,
    parent: stagedParent,
    folderName,
  })

  // Both directories are new, and the preview says so before submission.
  await expect(dialog.getByText(/Will create new folders:/)).toContainText(stagedParent)
  await expect(dialog.getByText(/Will create new folders:/)).toContainText(folderName)
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
  await openAddDialog(page)
  const dialog = page.getByRole('dialog')
  await openFolderBrowser(dialog)

  // The browser opens on the default parent, `playlists`.
  await expect(dialog.getByRole('navigation', { name: 'Folder path' })).toContainText('playlists')

  // One click on the videos root, then into its sibling — without ever
  // leaving the dialog or typing a path.
  await breadcrumbLinks(dialog).first().click()
  await folderEntry(dialog, 'channels').click()

  await expect(destinationPreview(dialog)).toContainText('/channels/')
  await expect(dialog.getByRole('navigation', { name: 'Folder path' })).toContainText('channels')
})

test('it should descend into an existing directory when the create-folder step names one', async ({
  page,
}) => {
  await page.goto('/')
  await openAddDialog(page)
  const dialog = page.getByRole('dialog')
  await openFolderBrowser(dialog)
  await breadcrumbLinks(dialog).first().click()

  // `channels` is seeded at daemon startup, so naming it here names something
  // that already exists.
  await dialog.getByRole('button', { name: 'New folder' }).click()
  await dialog.getByLabel('New folder name').fill('channels')
  await dialog.getByRole('button', { name: 'Use folder' }).click()

  await dialog.getByLabel('Folder name').fill('descended-target')
  await expect(destinationPreview(dialog)).toContainText('/channels/descended-target')
  // Descended into, not adopted as new: only the leaf is reported as created.
  await expect(dialog.getByText(/Will create a new folder: descended-target/)).toBeVisible()
  await expect(dialog.getByText(/Will create new folders:/)).toHaveCount(0)
})

test('it should block submission when the destination is already used by another playlist', async ({
  page,
}) => {
  test.skip(!PLAYLIST_ID, 'SMOKE_PLAYLIST_ID is not set')

  await page.goto('/')
  await openAddDialog(page)
  await submitPlaylist(page, { url: PLAYLIST_ID, name: PLAYLIST_NAME })
  const sidebarLink = page
    .locator('h3:text-is("Playlists") ~ ul')
    .getByRole('link', { name: PLAYLIST_NAME })
  await expect(sidebarLink).toBeVisible()

  // The same name derives the same folder under the same default parent.
  await openAddDialog(page)
  const dialog = await fillPlaylist(page, {
    url: 'not-a-real-playlist-id-conflict-check',
    name: PLAYLIST_NAME,
  })

  await expect(dialog.getByText(/Already used by/)).toBeVisible()
  await expect(dialog.getByRole('button', { name: /^Create Playlist/ })).toBeDisabled()
  await dialog.getByRole('button', { name: 'Close' }).click()

  await deleteItem(page, { section: 'Playlists', name: PLAYLIST_NAME })
})

test('it should reject a folder name containing a slash', async ({ page }) => {
  await page.goto('/')
  await openAddDialog(page)
  const dialog = page.getByRole('dialog')

  await dialog.getByLabel('Folder name').fill('kids/movies')

  await expect(dialog.getByText(/Folder name must not contain/)).toBeVisible()
  await expect(dialog.getByRole('button', { name: /^Create Playlist/ })).toBeDisabled()
})
