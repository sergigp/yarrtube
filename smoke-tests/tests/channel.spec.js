import { test, expect } from '@playwright/test'
import {
  openAddChannelDialog,
  fillChannel,
  destinationNotice,
} from '../helpers/addDialog.js'
import { waitForVideoStatus, assertVideoPlays } from '../helpers/video.js'
import {
  syncItem,
  deleteItem,
  markItemWatched,
  sectionRows,
  unwatchedBadge,
} from '../helpers/sidebar.js'

const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'
const VIDEO_LIMIT = process.env.SMOKE_CHANNEL_VIDEO_LIMIT ?? '1'
// The folder the dialog derives from a bare handle: without the `@`, slugified.
const CHANNEL_FOLDER = CHANNEL_HANDLE.replace(/^@+/, '')
  .toLowerCase()
  .replace(/[^a-z0-9]+/g, '-')
  .replace(/^-+|-+$/g, '')

test('channel lifecycle: add, download, play, resume, mark watched, sync, edit settings, invalid handle error, delete', async ({ page }) => {
  await page.goto('/')

  // Add via the sidebar's "Add channel" and confirm it lands in the sidebar. The
  // channel's display name comes from YouTube, not the handle we typed,
  // so identify it by section rather than by name.
  // Before submitting, the notice names the destination the videos land in.
  await openAddChannelDialog(page)
  const dialog = await fillChannel(page, { handle: CHANNEL_HANDLE, videoLimit: VIDEO_LIMIT })
  await expect(destinationNotice(dialog).locator('code')).toHaveText(
    new RegExp(`/channels/${CHANNEL_FOLDER}$`),
  )
  await dialog.getByRole('button', { name: /^Create Channel/ }).click()
  await dialog.waitFor({ state: 'hidden' })
  const sidebarLink = page.locator('h3:text-is("Channels") ~ ul li a').first()
  await expect(sidebarLink).toBeVisible()

  // The channel just added can't be added again, whatever the handle's case:
  // YouTube treats `@Name` and `@name` as the same channel.
  await openAddChannelDialog(page)
  await fillChannel(page, { handle: CHANNEL_HANDLE.toLowerCase() })
  await expect(destinationNotice(dialog)).toContainText('Already added as “')
  await expect(destinationNotice(dialog)).toHaveClass(/text-destructive/)
  await expect(dialog.getByRole('button', { name: /^Create Channel/ })).toBeDisabled()
  await dialog.getByRole('button', { name: 'Close' }).click()

  // Follow it into the detail view and wait for a video to download. No
  // fixed title is asserted, since the channel's newest video can change;
  // instead the actual title is read back to check the home feed later.
  await sidebarLink.click()
  await waitForVideoStatus(page, { status: 'DOWNLOADED', timeoutMs: 240_000 })
  const videoTitle = await page.locator('main h3').first().innerText()
  await assertVideoPlays(page)

  // Pausing halfway through reports the position, so reloading resumes from
  // it instead of from the start. Halfway stays below the 90% that would mark
  // the video watched, whatever the channel's newest video happens to last.
  const video = page.locator('video')
  const progressReported = page.waitForResponse(
    (response) => response.url().endsWith('/progress') && response.request().method() === 'POST',
  )
  await video.evaluate((el) => {
    el.currentTime = el.duration / 2
    el.pause()
  })
  expect((await progressReported).status()).toBe(200)
  await page.reload()
  await expect.poll(() => video.evaluate((el) => el.currentTime), { timeout: 15_000 }).toBeGreaterThan(0)

  // The downloaded, unwatched video shows as a badge on the channel's row
  // until the channel is marked watched from the sidebar.
  const channelRow = sectionRows(page, 'Channels').first()
  await expect(unwatchedBadge(channelRow)).toHaveText('1')
  await markItemWatched(page, { section: 'Channels', name: CHANNEL_HANDLE })
  await expect(unwatchedBadge(channelRow)).toHaveCount(0)

  // Sync from the sidebar completes without an error dialog/alert.
  await syncItem(page, { section: 'Channels', name: CHANNEL_HANDLE })

  // Change the video quality from the page header's "⋮" menu; reopening the
  // dialog shows the saved value (covers PATCH /channels/{handle}).
  const editChannel = async () => {
    await page.getByRole('button', { name: /^More actions for / }).click()
    await page.getByRole('menuitem', { name: 'Edit settings' }).click()
    return page.getByRole('dialog', { name: /^Edit .+ settings$/ })
  }
  let editDialog = await editChannel()
  await editDialog.getByRole('combobox', { name: /Video quality/ }).click()
  await page.getByRole('option', { name: 'Low' }).click()
  await editDialog.getByRole('button', { name: 'Save' }).click()
  await editDialog.waitFor({ state: 'hidden' })
  editDialog = await editChannel()
  await expect(editDialog.getByRole('combobox', { name: /Video quality/ })).toHaveText('Low')
  await editDialog.getByRole('button', { name: 'Close' }).click()

  // An invalid handle is explained in the notice and can't be submitted.
  await openAddChannelDialog(page)
  await fillChannel(page, { handle: 'this-handle-should-not-exist-abc123' })
  await expect(destinationNotice(dialog)).toHaveText(/must start with "@"/)
  await expect(destinationNotice(dialog)).toHaveClass(/text-destructive/)
  await expect(dialog.getByRole('button', { name: /^Create Channel/ })).toBeDisabled()
  await dialog.getByRole('button', { name: 'Close' }).click()

  // Delete from the sidebar; it disappears from both the sidebar and home feed.
  await deleteItem(page, { section: 'Channels', name: CHANNEL_HANDLE })
  await expect(sectionRows(page, 'Channels')).toHaveCount(0)
  // The sidebar also hides its list on a fetch error, so an empty list alone
  // wouldn't distinguish "deleted" from "broken" — check for the explicit
  // empty-state text instead.
  await expect(page.locator('h3:text-is("Channels") ~ p', { hasText: 'None tracked yet.' })).toBeVisible()
  await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
  await expect(page.getByText(videoTitle, { exact: true })).toHaveCount(0)
})
