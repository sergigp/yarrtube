import { test, expect } from '@playwright/test'
import {
  openAddPlaylistDialog,
  submitPlaylist,
  fillPlaylist,
  destinationNotice,
} from '../helpers/addDialog.js'
import { waitForVideoStatus, assertVideoPlays } from '../helpers/video.js'
import { syncItem, deleteItem, sectionRows } from '../helpers/sidebar.js'

const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
const PLAYLIST_NAME = process.env.SMOKE_PLAYLIST_NAME ?? 'test'

test('playlist lifecycle: add, download, play, sync, duplicate error, delete', async ({ page }) => {
  test.skip(!PLAYLIST_ID, 'SMOKE_PLAYLIST_ID is not set')

  await page.goto('/')

  // Add via the sidebar's "Add playlist" and confirm it lands in the sidebar.
  await openAddPlaylistDialog(page)
  await submitPlaylist(page, { url: PLAYLIST_ID })
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

  // Pausing halfway through records the position. The playlist's video (Big
  // Buck Bunny, about 10 minutes) is long enough that halfway is past the 30
  // seconds a video must reach to count as started, so the home view offers
  // it to continue watching.
  const progressReported = page.waitForResponse(
    (response) => response.url().endsWith('/progress') && response.request().method() === 'POST',
  )
  await page.locator('video').evaluate((el) => {
    el.currentTime = el.duration / 2
    el.pause()
  })
  expect((await progressReported).status()).toBe(200)

  // Same video, with thumbnail + duration, on the home feed.
  await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
  const continueWatching = page.locator('section', {
    has: page.getByRole('heading', { name: 'Continue watching' }),
  })
  await expect(continueWatching.getByText(videoTitle, { exact: true })).toBeVisible()
  await expect(continueWatching.getByRole('progressbar', { name: 'Watch progress' })).toBeVisible()
  // A video shows in one home section only, so it isn't repeated below.
  const latestVideos = page.locator('section', {
    has: page.getByRole('heading', { name: 'Latest videos' }),
  })
  await expect(latestVideos).toBeVisible()
  await expect(latestVideos.getByText(videoTitle, { exact: true })).toHaveCount(0)

  // The home sections render in order: the two short shelves, then the
  // longer "Latest videos". "Quick watches" only shows when a short video is
  // downloaded, which the smoke playlist doesn't guarantee.
  const homeSections = ['Continue watching', 'Quick watches', 'Latest videos']
  await expect
    .poll(async () => {
      const headings = await page.locator('main h2').allInnerTexts()
      return headings.filter((heading) => homeSections.includes(heading))
    })
    .toEqual(
      expect.arrayContaining(['Continue watching', 'Latest videos']),
    )
  const headings = (await page.locator('main h2').allInnerTexts()).filter((heading) =>
    homeSections.includes(heading),
  )
  expect(headings).toEqual(homeSections.filter((section) => headings.includes(section)))
  const homeCard = page.locator('main').getByRole('link').filter({ has: page.locator('img') }).first()
  await expect(homeCard).toBeVisible()
  await expect(homeCard.getByText(/^\d+:\d{2}(:\d{2})?$/)).toBeVisible()

  // Sync from the sidebar completes without an error dialog/alert.
  await syncItem(page, { section: 'Playlists', name: PLAYLIST_NAME })

  // Adding the same playlist again is caught in the dialog, before
  // submission: the notice names what it was added as and the submit button
  // stays disabled.
  await openAddPlaylistDialog(page)
  const dialog = await fillPlaylist(page, { url: PLAYLIST_ID })
  await expect(destinationNotice(dialog)).toHaveText(`Already added as “${PLAYLIST_NAME}”`)
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
