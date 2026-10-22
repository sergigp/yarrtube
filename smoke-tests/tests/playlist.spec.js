import { test, expect } from '../helpers/fixtures.js'
import {
  openAddPlaylistDialog,
  submitPlaylist,
  fillPlaylist,
  destinationNotice,
  slugOf,
} from '../helpers/addDialog.js'
import { waitForVideoStatus, assertVideoPlays, assertSubtitlesShow } from '../helpers/video.js'
import { expectEpisodeOnDisk, expectGoneFromDisk } from '../helpers/library.js'
import { expectPlexScanned } from '../helpers/plex.js'
import {
  syncItem,
  deleteItem,
  sectionRows,
  includeItemInHome,
  markItemWatched,
  sectionList,
  setItemUnwatchedCountHidden,
  unwatchedBadge,
} from '../helpers/sidebar.js'
import { expectPendingSync } from '../helpers/tasks.js'

const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
const PLAYLIST_NAME = process.env.SMOKE_PLAYLIST_NAME ?? 'test'
const PLAYLIST_PATH = `playlists/${slugOf(PLAYLIST_NAME)}`
// The badge of an in-progress thumbnail: "<1m left", "8m left", "1h 5m left".
const TIME_LEFT = /^(<1m|\d+m|\d+h( \d+m)?) left$/

function escapeRegExp(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

test('playlist lifecycle: add, download, play, sync, duplicate error, delete', async ({ page }) => {
  await page.goto('/')

  const sidebarLink = sectionList(page, 'Playlists').getByRole('link', { name: PLAYLIST_NAME })
  const playlistRow = sectionRows(page, 'Playlists').filter({ hasText: PLAYLIST_NAME })
  const continueWatching = page.locator('section', {
    has: page.getByRole('heading', { name: 'Continue watching' }),
  })

  await test.step('add it from the sidebar', async () => {
    // Add via the sidebar's "Add playlist" and confirm it lands in the sidebar.
    await openAddPlaylistDialog(page)
    await submitPlaylist(page, { url: PLAYLIST_ID })
    await expect(sidebarLink).toBeVisible()
  })

  const videoTitle = await test.step('wait for the download, on disk and scanned by Plex', async () => {
    // Follow it into the detail view and wait for the download to finish.
    await sidebarLink.click()
    await waitForVideoStatus(page, { status: 'DOWNLOADED', timeoutMs: 240_000 })
    const videoTitle = await page.locator('main h3').first().innerText()

    // On disk it is the playlist's first episode, laid out for Plex.
    await expectEpisodeOnDisk({
      showPath: PLAYLIST_PATH,
      kind: 'playlist',
      showTitle: PLAYLIST_NAME,
      videoTitle,
    })
    // And Plex was asked to scan it in.
    await expectPlexScanned(page.request, PLAYLIST_PATH)
    return videoTitle
  })

  await test.step('show its unwatched badge, hide it and show it again', async () => {
    // The downloaded, unwatched video shows as a badge on the playlist's row;
    // reloading skips waiting for the sidebar's periodic refresh. Hiding the
    // count removes the badge and showing it brings it back (covers PATCH
    // /playlists/{id} with hide_unwatched_count).
    await page.reload()
    await expect(unwatchedBadge(playlistRow)).toHaveText('1')
    await setItemUnwatchedCountHidden(page, { section: 'Playlists', name: PLAYLIST_NAME, hidden: true })
    await expect(unwatchedBadge(playlistRow)).toHaveCount(0)
    await setItemUnwatchedCountHidden(page, { section: 'Playlists', name: PLAYLIST_NAME, hidden: false })
    await expect(unwatchedBadge(playlistRow)).toHaveText('1')
  })

  await test.step('play it and record the position', async () => {
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
  })

  await test.step('show its subtitles', async () => {
    await assertSubtitlesShow(page)
  })

  await test.step('show it on the home feed', async () => {
    // Same video, with thumbnail + duration, on the home feed.
    await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
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
    // The first card is the half-watched one under "Continue watching": it
    // shows the time left instead of the duration.
    const homeCard = page.locator('main').getByRole('link').filter({ has: page.locator('img') }).first()
    await expect(homeCard).toBeVisible()
    await expect(homeCard.getByText(TIME_LEFT)).toBeVisible()
  })

  await test.step('exclude it from home and include it again', async () => {
    // Excluding the playlist from home through a card's menu takes its cards
    // off home; including it again from the sidebar brings them back.
    const excluded = page.waitForResponse(
      (response) => response.request().method() === 'PATCH' && response.url().includes('/playlists/'),
    )
    await continueWatching.getByRole('button', { name: `Actions for ${videoTitle}` }).click()
    await page.getByRole('menuitem', { name: `Exclude "${PLAYLIST_NAME}" from home` }).click()
    expect((await excluded).status()).toBe(200)
    await expect(page.locator('main').getByText(videoTitle, { exact: true })).toHaveCount(0)
    await includeItemInHome(page, { section: 'Playlists', name: PLAYLIST_NAME })
    await expect(continueWatching.getByText(videoTitle, { exact: true })).toBeVisible()
  })

  await test.step("show its progress in the playlist's video list", async () => {
    // The half-watched video shows the same progress bar and time left in its
    // playlist's list as on home, not only under "Continue watching".
    await sectionList(page, 'Playlists').getByRole('link', { name: PLAYLIST_NAME }).click()
    const row = page.getByRole('button', { name: new RegExp(`^(?!Actions for).*${escapeRegExp(videoTitle)}$`) })
    await expect(row.getByRole('progressbar', { name: 'Watch progress' })).toBeVisible()
    await expect(row.getByText(TIME_LEFT)).toBeVisible()
  })

  await test.step('sync changes nothing when up to date', async () => {
    // Sync from the sidebar completes without an error dialog/alert, and
    // syncing an up-to-date playlist changes nothing: the same videos, still
    // downloaded, and nothing queued to download again.
    const videosUrl = `/api/playlists/${encodeURIComponent(PLAYLIST_ID)}/videos`
    const videoState = async () =>
      (await (await page.request.get(videosUrl)).json()).map(({ id, status, filename }) => ({ id, status, filename }))
    const videosBeforeSync = await videoState()
    await syncItem(page, { section: 'Playlists', name: PLAYLIST_NAME })
    expect(await videoState()).toEqual(videosBeforeSync)
    const tasks = await (await page.request.get('/api/tasks')).json()
    expect(tasks.filter((task) => task.task_type === 'download_video')).toEqual([])
  })

  await test.step('edit the check interval', async () => {
    // Change how often the playlist is checked from its page header's "⋮"
    // menu; reopening the dialog shows the saved value (covers PATCH
    // /playlists/{id} with an interval).
    await sectionList(page, 'Playlists').getByRole('link', { name: PLAYLIST_NAME }).click()
    const editPlaylist = async () => {
      await page.getByRole('button', { name: /^More actions for / }).click()
      await page.getByRole('menuitem', { name: 'Edit settings' }).click()
      return page.getByRole('dialog', { name: `Edit ${PLAYLIST_NAME} settings` })
    }
    let editDialog = await editPlaylist()
    await expect(editDialog.getByLabel('Check for new videos every')).toHaveValue('3')
    await editDialog.getByLabel('Check for new videos every').fill('30')
    await editDialog.getByRole('combobox', { name: 'Interval unit' }).click()
    await page.getByRole('option', { name: 'minutes' }).click()
    const intervalSaved = page.waitForResponse(
      (response) => response.request().method() === 'PATCH' && response.url().includes('/playlists/'),
    )
    await editDialog.getByRole('button', { name: 'Save' }).click()
    expect((await intervalSaved).status()).toBe(200)
    await editDialog.waitFor({ state: 'hidden' })
    editDialog = await editPlaylist()
    await expect(editDialog.getByLabel('Check for new videos every')).toHaveValue('30')
    await expect(editDialog.getByRole('combobox', { name: 'Interval unit' })).toHaveText('minutes')
    await editDialog.getByRole('button', { name: 'Close' }).click()

    // The shorter interval brought the playlist's next sync forward: it was
    // due in 3 hours, now in about 30 minutes.
    await expectPendingSync(page, { label: `Syncing playlist ${PLAYLIST_NAME}`, due: /^in (29|30)m$/ })
  })

  await test.step('run its sync now from the Tasks view', async () => {
    // The sync can also be run straight away from its row (covers the Tasks
    // view's "Run now").
    const syncRow = page.locator('main li', { hasText: `Syncing playlist ${PLAYLIST_NAME}` })
    const ranNow = page.waitForResponse(
      (response) => response.url().endsWith('/reconcile') && response.request().method() === 'POST',
    )
    await syncRow.getByRole('button', { name: /^Run now: / }).click()
    expect((await ranNow).ok()).toBe(true)
    await expect(syncRow.getByRole('button', { name: /^Run now: / })).toBeEnabled()
    await expect(syncRow.getByText(/^Sync failed/)).toHaveCount(0)
  })

  await test.step('mark it watched while playing', async () => {
    // Playing past the watched threshold marks the video watched without
    // interrupting playback: once the list shows it watched, the player is
    // still playing where it was, not rewound and paused.
    await sectionList(page, 'Playlists').getByRole('link', { name: PLAYLIST_NAME }).click()
    await assertVideoPlays(page)
    const player = page.locator('video')
    const markedWatched = page.waitForResponse(
      async (response) =>
        response.url().endsWith('/progress') &&
        response.request().method() === 'POST' &&
        (await response.json()).watched === true,
      { timeout: 30_000 },
    )
    const nearEnd = await player.evaluate(async (el) => {
      el.currentTime = el.duration * 0.92
      await el.play()
      return el.currentTime
    })
    await markedWatched
    await expect(page.locator('main ul li').getByLabel('Watched')).toBeVisible({ timeout: 10_000 })
    expect(await player.evaluate((el) => el.paused)).toBe(false)
    expect(await player.evaluate((el) => el.currentTime)).toBeGreaterThan(nearEnd)
  })

  await test.step('mark it all watched from its row menu', async () => {
    // Covers POST /playlists/{id}/watched; with its only video watched, the
    // row shows no badge.
    await markItemWatched(page, { section: 'Playlists', name: PLAYLIST_NAME })
    await expect(unwatchedBadge(playlistRow)).toHaveCount(0)
  })

  await test.step('reject adding it twice', async () => {
    // Adding the same playlist again is caught in the dialog, before
    // submission: the notice names what it was added as and the submit button
    // stays disabled.
    await openAddPlaylistDialog(page)
    const dialog = await fillPlaylist(page, { url: PLAYLIST_ID })
    await expect(destinationNotice(dialog)).toHaveText(`Already added as “${PLAYLIST_NAME}”`)
    await expect(dialog.getByRole('button', { name: /^Create Playlist/ })).toBeDisabled()
    await dialog.getByRole('button', { name: 'Close' }).click()
  })

  await test.step('delete it', async () => {
    // Delete from the sidebar; it disappears from the sidebar, the home feed
    // (which, the library being otherwise empty, now has nothing to show) and
    // the disk.
    await deleteItem(page, { section: 'Playlists', name: PLAYLIST_NAME })
    await expect(sectionRows(page, 'Playlists')).toHaveCount(0)
    await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
    await expect(page.getByText('No videos synced yet.')).toBeVisible()
    await expectGoneFromDisk(PLAYLIST_PATH)
  })
})
