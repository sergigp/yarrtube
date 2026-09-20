import { test, expect } from '@playwright/test'
import { openAddDialog, submitChannel, dialogErrorText } from '../helpers/addDialog.js'
import { waitForVideoStatus, assertVideoPlays } from '../helpers/video.js'
import { syncItem, deleteItem } from '../helpers/sidebar.js'

const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'
const VIDEO_LIMIT = process.env.SMOKE_CHANNEL_VIDEO_LIMIT ?? '1'

test('channel lifecycle: add, download, play, sync, invalid handle error, delete', async ({ page }) => {
  await page.goto('/')

  // Add via the Add dialog and confirm it lands in the sidebar. The
  // channel's display name comes from YouTube, not the handle we typed,
  // so identify it by section rather than by name.
  await openAddDialog(page)
  await submitChannel(page, { handle: CHANNEL_HANDLE, videoLimit: VIDEO_LIMIT })
  const sidebarLink = page.locator('h3:text-is("Channels") ~ ul li a').first()
  await expect(sidebarLink).toBeVisible()

  // Follow it into the detail view and wait for a video to download. No
  // fixed title is asserted, since the channel's newest video can change;
  // instead the actual title is read back to check the home feed later.
  await sidebarLink.click()
  await waitForVideoStatus(page, { status: 'DOWNLOADED', timeoutMs: 120_000 })
  const videoTitle = await page.locator('main h3').first().innerText()
  await assertVideoPlays(page)

  // Sync from the sidebar completes without an error dialog/alert.
  await syncItem(page, { section: 'Channels', name: CHANNEL_HANDLE })

  // An invalid handle surfaces a visible error in the dialog.
  await openAddDialog(page)
  await submitChannel(page, { handle: 'this-handle-should-not-exist-abc123' })
  await expect(dialogErrorText(page)).toBeVisible()
  await page.getByRole('dialog').getByRole('button', { name: 'Close' }).click()

  // Delete from the sidebar; it disappears from both the sidebar and home feed.
  await deleteItem(page, { section: 'Channels', name: CHANNEL_HANDLE })
  await expect(page.locator('h3:text-is("Channels") ~ ul li')).toHaveCount(0)
  await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
  await expect(page.getByText(videoTitle, { exact: true })).toHaveCount(0)
})
