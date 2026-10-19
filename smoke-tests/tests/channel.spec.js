import { test, expect } from '../helpers/fixtures.js'
import {
  openAddChannelDialog,
  fillChannel,
  destinationNotice,
  slugOf,
} from '../helpers/addDialog.js'
import { waitForVideoStatus, assertVideoPlays, assertSubtitlesShow } from '../helpers/video.js'
import {
  syncItem,
  deleteItem,
  markItemWatched,
  sectionRows,
  unwatchedBadge,
} from '../helpers/sidebar.js'
import { expectPendingSync } from '../helpers/tasks.js'
import { expectEpisodeOnDisk, expectGoneFromDisk } from '../helpers/library.js'
import { expectPlexScanned } from '../helpers/plex.js'

const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'
const VIDEO_LIMIT = process.env.SMOKE_CHANNEL_VIDEO_LIMIT ?? '1'
const CHANNEL_PATH = `channels/${slugOf(CHANNEL_HANDLE)}`

test('channel lifecycle: add, download, play, resume, mark watched, sync, edit settings, invalid handle error, delete', async ({ page }) => {
  await page.goto('/')

  const dialog = page.getByRole('dialog')
  const sidebarLink = page.locator('h3:text-is("Channels") ~ ul li a').first()
  const channelRow = sectionRows(page, 'Channels').first()

  await test.step('add it from the sidebar', async () => {
    // Add via the sidebar's "Add channel" and confirm it lands in the sidebar. The
    // channel's display name comes from YouTube, not the handle we typed,
    // so identify it by section rather than by name.
    // Before submitting, the notice names the destination the videos land in.
    await openAddChannelDialog(page)
    await fillChannel(page, { handle: CHANNEL_HANDLE, videoLimit: VIDEO_LIMIT })
    await expect(destinationNotice(dialog).locator('code')).toHaveText(
      new RegExp(`/${CHANNEL_PATH}$`),
    )
    await dialog.getByRole('button', { name: /^Create Channel/ }).click()
    await dialog.waitFor({ state: 'hidden' })
    await expect(sidebarLink).toBeVisible()
  })

  await test.step('reject adding it twice, whatever the case', async () => {
    // The channel just added can't be added again, whatever the handle's case:
    // YouTube treats `@Name` and `@name` as the same channel.
    await openAddChannelDialog(page)
    await fillChannel(page, { handle: CHANNEL_HANDLE.toLowerCase() })
    await expect(destinationNotice(dialog)).toContainText('Already added as “')
    await expect(destinationNotice(dialog)).toHaveClass(/text-destructive/)
    await expect(dialog.getByRole('button', { name: /^Create Channel/ })).toBeDisabled()
    await dialog.getByRole('button', { name: 'Close' }).click()
  })

  const { videoTitle, channelName } = await test.step('wait for the download, on disk and scanned by Plex', async () => {
    // Follow it into the detail view and wait for a video to download. No
    // fixed title is asserted, since the channel's newest video can change;
    // instead the actual title is read back to check the home feed later.
    await sidebarLink.click()
    await waitForVideoStatus(page, { status: 'DOWNLOADED', timeoutMs: 240_000 })
    const videoTitle = await page.locator('main h3').first().innerText()
    const [{ name: channelName }] = await (await page.request.get('/api/channels')).json()
    await expectEpisodeOnDisk({ showPath: CHANNEL_PATH, kind: 'channel', showTitle: channelName, videoTitle })
    await expectPlexScanned(page.request, CHANNEL_PATH)
    await assertVideoPlays(page)
    return { videoTitle, channelName }
  })

  await test.step('show its subtitles', async () => {
    await assertSubtitlesShow(page)
  })

  await test.step('resume from the recorded position', async () => {
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
  })

  await test.step('mark its video watched', async () => {
    // The downloaded, unwatched video shows as a badge on the channel's row
    // until the video is marked watched from its own actions menu (covers
    // POST /videos/{id}/watched).
    await expect(unwatchedBadge(channelRow)).toHaveText('1')
    const markedOne = page.waitForResponse(
      (response) => response.url().endsWith('/watched') && response.request().method() === 'POST',
    )
    await page.getByRole('button', { name: `Actions for ${videoTitle}` }).first().click()
    await page.getByRole('menuitem', { name: 'Mark as watched' }).click()
    expect(new URL((await markedOne).url()).pathname).toMatch(/^\/api\/videos\/[^/]+\/watched$/)
    await expect(unwatchedBadge(channelRow)).toHaveCount(0)
    await expect(page.locator('main ul li').getByLabel('Watched')).toBeVisible()
  })

  await test.step('mark the whole channel watched', async () => {
    // Marking the whole channel watched from the sidebar succeeds too; with
    // its only video already watched there is nothing left to change.
    await markItemWatched(page, { section: 'Channels', name: CHANNEL_HANDLE })
    await expect(unwatchedBadge(channelRow)).toHaveCount(0)
  })

  await test.step('sync it', async () => {
    // Sync from the sidebar completes without an error dialog/alert.
    await syncItem(page, { section: 'Channels', name: CHANNEL_HANDLE })
  })

  await test.step('edit its quality and check interval', async () => {
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

    // Shorten the check interval to 1 hour from the same dialog: the channel's
    // next sync, due in 3 hours, is brought forward to about an hour.
    await expect(editDialog.getByLabel('Check for new videos every')).toHaveValue('3')
    await editDialog.getByLabel('Check for new videos every').fill('1')
    const intervalSaved = page.waitForResponse(
      (response) => response.request().method() === 'PATCH' && response.url().includes('/channels/'),
    )
    await editDialog.getByRole('button', { name: 'Save' }).click()
    expect((await intervalSaved).status()).toBe(200)
    await editDialog.waitFor({ state: 'hidden' })
    await expectPendingSync(page, { label: `Syncing channel ${channelName}`, due: /^in (59m|1h)$/ })
  })

  await test.step('reject an invalid handle', async () => {
    // An invalid handle is explained in the notice and can't be submitted.
    await openAddChannelDialog(page)
    await fillChannel(page, { handle: 'this-handle-should-not-exist-abc123' })
    await expect(destinationNotice(dialog)).toHaveText(/must start with "@"/)
    await expect(destinationNotice(dialog)).toHaveClass(/text-destructive/)
    await expect(dialog.getByRole('button', { name: /^Create Channel/ })).toBeDisabled()
    await dialog.getByRole('button', { name: 'Close' }).click()
  })

  await test.step('delete it', async () => {
    // Delete from the sidebar; it disappears from the sidebar, the home feed
    // and the disk.
    await deleteItem(page, { section: 'Channels', name: CHANNEL_HANDLE })
    await expect(sectionRows(page, 'Channels')).toHaveCount(0)
    // The sidebar also hides its list on a fetch error, so an empty list alone
    // wouldn't distinguish "deleted" from "broken" — check for the explicit
    // empty-state text instead.
    await expect(page.locator('h3:text-is("Channels") ~ p', { hasText: 'None tracked yet.' })).toBeVisible()
    await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
    // The library being otherwise empty, home has nothing left to show.
    await expect(page.getByText('No videos synced yet.')).toBeVisible()
    await expectGoneFromDisk(CHANNEL_PATH)
  })
})
