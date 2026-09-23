import { test, expect } from '@playwright/test'
import { openAddDialog, submitPlaylist, fillPlaylist } from '../helpers/addDialog.js'
import { waitForVideoStatus, assertVideoPlays } from '../helpers/video.js'
import { syncItem, deleteItem, sectionRows } from '../helpers/sidebar.js'

const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
const PLAYLIST_NAME = process.env.SMOKE_PLAYLIST_NAME ?? 'yarrtube smoke tests'

test('playlist lifecycle: add, download, play, sync, duplicate error, delete', async ({ page }) => {
  test.skip(!PLAYLIST_ID, 'SMOKE_PLAYLIST_ID is not set')

  await page.goto('/')

  // Add via the Add dialog and confirm it lands in the sidebar.
  await openAddDialog(page)
  await submitPlaylist(page, { url: PLAYLIST_ID, name: PLAYLIST_NAME })
  const sidebarLink = page.locator('h3:text-is("Playlists") ~ ul').getByRole('link', { name: PLAYLIST_NAME })
  await expect(sidebarLink).toBeVisible()

  // Follow it into the detail view and wait for the download to finish.
  await sidebarLink.click()
  await waitForVideoStatus(page, { status: 'DOWNLOADED', timeoutMs: 240_000 })
  const videoTitle = await page.locator('main h3').first().innerText()

  // Thumbnail + duration render in the detail view's video list. Scoped to
  // `main` so a leftover sidebar row (e.g. a channel avatar, if a previous
  // test left one behind) can never be picked up instead.
  const detailListItem = page.locator('main ul li', { has: page.locator('img') }).first()
  await expect(detailListItem.locator('img')).toBeVisible()
  await expect(detailListItem.getByText(/^\d+:\d{2}(:\d{2})?$/)).toBeVisible()

  await assertVideoPlays(page)

  // Same video, with thumbnail + duration, on the home feed.
  await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
  const homeCard = page.locator('main').getByRole('link').filter({ has: page.locator('img') }).first()
  await expect(homeCard).toBeVisible()
  await expect(homeCard.getByText(/^\d+:\d{2}(:\d{2})?$/)).toBeVisible()

  // Sync from the sidebar completes without an error dialog/alert.
  await syncItem(page, { section: 'Playlists', name: PLAYLIST_NAME })

  // The same name auto-derives the same folder under the same default
  // parent as the already-tracked playlist. That conflict is now caught in
  // the dialog, before submission, so the preview reports it and the submit
  // button stays disabled — there is no rejected request to recover from.
  await openAddDialog(page)
  const dialog = await fillPlaylist(page, {
    url: 'not-a-real-playlist-id-path-conflict-check',
    name: PLAYLIST_NAME,
  })
  await expect(dialog.getByText(/Already used by/)).toBeVisible()
  await expect(dialog.getByRole('button', { name: /^Create Playlist/ })).toBeDisabled()
  await dialog.getByRole('button', { name: 'Close' }).click()

  // Delete from the sidebar; it disappears from both the sidebar and home feed.
  await deleteItem(page, { section: 'Playlists', name: PLAYLIST_NAME })
  await expect(sectionRows(page, 'Playlists')).toHaveCount(0)
  await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
  // The home feed only ever renders video titles, never the playlist's own
  // name, so check the actual video is gone rather than a string that would
  // never have appeared there in the first place.
  await expect(page.getByText(videoTitle, { exact: true })).toHaveCount(0)
})
