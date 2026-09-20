import { test, expect } from '@playwright/test'
import { openAddDialog, submitPlaylist, dialogErrorText } from '../helpers/addDialog.js'
import { waitForVideoStatus, assertVideoPlays } from '../helpers/video.js'
import { syncItem, deleteItem } from '../helpers/sidebar.js'

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
  await waitForVideoStatus(page, { status: 'DOWNLOADED', timeoutMs: 120_000 })

  // Thumbnail + duration render in the detail view's video list.
  const detailListItem = page.locator('ul li', { has: page.locator('img') }).first()
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

  // A different (fake) source id with the same name auto-derives the same
  // path as the already-tracked playlist, tripping the path-conflict error
  // and auto-expanding Advanced options. Resubmitting the *same* id instead
  // would just be treated as an idempotent "already exists", not a conflict.
  await openAddDialog(page)
  await submitPlaylist(page, { url: 'not-a-real-playlist-id-path-conflict-check', name: PLAYLIST_NAME })
  await expect(dialogErrorText(page)).toBeVisible()
  await expect(page.getByRole('dialog').getByRole('button', { name: /Advanced options/ })).toHaveAttribute(
    'aria-expanded',
    'true',
  )
  await page.getByRole('dialog').getByRole('button', { name: 'Close' }).click()

  // Delete from the sidebar; it disappears from both the sidebar and home feed.
  await deleteItem(page, { section: 'Playlists', name: PLAYLIST_NAME })
  await expect(page.locator('h3:text-is("Playlists") ~ ul').getByText(PLAYLIST_NAME)).toHaveCount(0)
  await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
  await expect(page.getByRole('link').filter({ hasText: PLAYLIST_NAME })).toHaveCount(0)
})
