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
import { addPlaylist } from '../helpers/seed.js'
import { PLAYLIST, SLOW_PLAYLIST } from '../helpers/fakeYoutube.js'
import { expectImageLoaded } from '../helpers/media.js'
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

const PLAYLIST_ID = PLAYLIST.id
const PLAYLIST_NAME = PLAYLIST.title
const PLAYLIST_PATH = `playlists/${slugOf(PLAYLIST_NAME)}`
// The badge of an in-progress thumbnail: "<1m left", "8m left", "1h 5m left".
const TIME_LEFT = /^(<1m|\d+m|\d+h( \d+m)?) left$/

function escapeRegExp(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function playlistRow(page) {
  return sectionRows(page, 'Playlists').filter({ hasText: PLAYLIST_NAME })
}

function continueWatching(page) {
  return page.locator('section', { has: page.getByRole('heading', { name: 'Continue watching' }) })
}

/**
 * Pauses the selected video at 40 s, which records the position: past the
 * 30 s a video must reach to count as started (so the home view offers it to
 * continue watching) and well short of the 90% that marks it watched, on the
 * 61 s fixture clip.
 */
async function recordPosition(page) {
  const progressReported = page.waitForResponse(
    (response) => response.url().endsWith('/progress') && response.request().method() === 'POST',
  )
  await page.locator('video').evaluate((el) => {
    el.currentTime = 40
    el.pause()
  })
  expect((await progressReported).status()).toBe(200)
}

test('playlist golden path: add, download, on disk, scanned by Plex, badge, delete', async ({ page }) => {
  await page.goto('/')

  const sidebarLink = sectionList(page, 'Playlists').getByRole('link', { name: PLAYLIST_NAME })

  await test.step('add it from the sidebar', async () => {
    // Add via the sidebar's "Add playlist" and confirm it lands in the sidebar.
    await openAddPlaylistDialog(page)
    await submitPlaylist(page, { url: PLAYLIST_ID })
    await expect(sidebarLink).toBeVisible()
  })

  await test.step('wait for the download, on disk and scanned by Plex', async () => {
    // Follow it into the detail view and wait for the download to finish.
    await sidebarLink.click()
    await waitForVideoStatus(page, { status: 'DOWNLOADED', timeoutMs: 30_000 })
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
  })

  await test.step('unwatched badge appears via periodic refresh without reload', async () => {
    // The downloaded, unwatched video shows as a badge on the playlist's row,
    // brought in by the sidebar's periodic refresh (every 5s while the
    // playlist has work in progress, every 60s after), with no reload.
    // Up to 5 s for the work in progress to finish, then up to 60 s for the
    // next refresh, plus margin.
    await expect(unwatchedBadge(playlistRow(page))).toHaveText('1', { timeout: 90_000 })
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

test.describe('with a downloaded playlist', () => {
  // Each test adds its own: against the fake YouTube it downloads in about
  // a second.
  let seeded
  test.beforeEach(async ({ request }) => {
    const playlist = await addPlaylist(request, PLAYLIST)
    seeded = { ...playlist, video: playlist.videos[0] }
  })

  const openPlaylist = (page) => page.goto(`/playlists/${encodeURIComponent(seeded.id)}`)

  test('hides its unwatched badge and shows it again', async ({ page }) => {
    // Covers PATCH /playlists/{id} with hide_unwatched_count.
    await openPlaylist(page)
    await expect(unwatchedBadge(playlistRow(page))).toHaveText('1')
    await setItemUnwatchedCountHidden(page, { section: 'Playlists', name: PLAYLIST_NAME, hidden: true })
    await expect(unwatchedBadge(playlistRow(page))).toHaveCount(0)
    await setItemUnwatchedCountHidden(page, { section: 'Playlists', name: PLAYLIST_NAME, hidden: false })
    await expect(unwatchedBadge(playlistRow(page))).toHaveText('1')
  })

  test('lists it with thumbnail and duration, plays it and records the position', async ({ page }) => {
    // Scoped to `main` so a sidebar row's avatar can never be picked up instead.
    await openPlaylist(page)
    const detailListItem = page.locator('main ul li', { has: page.locator('img') }).first()
    await expectImageLoaded(detailListItem.locator('img'))
    await expect(detailListItem.getByText(/^\d+:\d{2}(:\d{2})?$/)).toBeVisible()

    await assertVideoPlays(page)
    await recordPosition(page)
  })

  test('shows its subtitles', async ({ page }) => {
    await openPlaylist(page)
    await assertSubtitlesShow(page)
  })

  test('shows it on the home feed', async ({ page }) => {
    const videoTitle = seeded.video.title
    await openPlaylist(page)
    await assertVideoPlays(page)
    await recordPosition(page)

    // Same video, with thumbnail + duration, on the home feed.
    await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
    await expect(continueWatching(page).getByText(videoTitle, { exact: true })).toBeVisible()
    await expect(continueWatching(page).getByRole('progressbar', { name: 'Watch progress' })).toBeVisible()
    // A video shows in one home section only, so it isn't repeated below.
    await expect(page.locator('main').getByText(videoTitle, { exact: true })).toHaveCount(1)

    // The home sections render in order: the two short shelves, then the
    // longer "Latest videos". "Quick watches" only shows when a short video is
    // downloaded, which the e2e playlist doesn't guarantee, and "Latest
    // videos" hides when every other video is shown above it.
    const homeSections = ['Continue watching', 'Quick watches', 'Latest videos']
    await expect
      .poll(async () => {
        const headings = await page.locator('main h2').allInnerTexts()
        return headings.filter((heading) => homeSections.includes(heading))
      })
      .toEqual(expect.arrayContaining(['Continue watching']))
    const headings = (await page.locator('main h2').allInnerTexts()).filter((heading) =>
      homeSections.includes(heading),
    )
    expect(headings).toEqual(homeSections.filter((section) => headings.includes(section)))
    // The first card is the half-watched one under "Continue watching": it
    // shows the time left instead of the duration.
    const homeCard = page.locator('main section li').filter({ has: page.locator('img') }).first()
    await expectImageLoaded(homeCard.locator('img').first())
    await expect(homeCard.getByText(TIME_LEFT)).toBeVisible()
  })

  test('excludes it from home and includes it again', async ({ page }) => {
    // Excluding the playlist from home through a card's menu takes its cards
    // off home; including it again from the sidebar brings them back.
    const videoTitle = seeded.video.title
    await openPlaylist(page)
    await assertVideoPlays(page)
    await recordPosition(page)
    await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
    await expect(continueWatching(page).getByText(videoTitle, { exact: true })).toBeVisible()

    const excluded = page.waitForResponse(
      (response) => response.request().method() === 'PATCH' && response.url().includes('/playlists/'),
    )
    await continueWatching(page).getByRole('button', { name: `Actions for ${videoTitle}` }).click()
    await page.getByRole('menuitem', { name: `Exclude "${PLAYLIST_NAME}" from home` }).click()
    expect((await excluded).status()).toBe(200)
    await expect(page.locator('main').getByText(videoTitle, { exact: true })).toHaveCount(0)
    await includeItemInHome(page, { section: 'Playlists', name: PLAYLIST_NAME })
    await expect(continueWatching(page).getByText(videoTitle, { exact: true })).toBeVisible()
  })

  test('opens a home card\'s video from anywhere on the card, and its playlist from its icon', async ({ page }) => {
    const videoTitle = seeded.video.title
    await page.goto('/')
    const card = page.locator('main li').filter({ hasText: videoTitle }).first()

    // The right end of the meta line holds no control of its own.
    const meta = await card.locator('[data-slot="card-meta"]').boundingBox()
    const cardBox = await card.boundingBox()
    await card.click({ position: { x: cardBox.width - 40, y: meta.y - cardBox.y + meta.height / 2 } })
    await expect(page).toHaveURL(new RegExp(`/playlists/${encodeURIComponent(seeded.id)}\\?video=`))

    await page.goBack()
    await card.getByRole('link', { name: PLAYLIST_NAME }).first().click()
    await expect(page).toHaveURL(new RegExp(`/playlists/${encodeURIComponent(seeded.id)}$`))
  })

  test("shows its progress in the playlist's video list", async ({ page }) => {
    // The half-watched video shows the same progress bar and time left in its
    // playlist's list as on home, not only under "Continue watching".
    const videoTitle = seeded.video.title
    await openPlaylist(page)
    await assertVideoPlays(page)
    await recordPosition(page)
    // The row's name ends with the title, then how long ago it was published.
    const row = page.getByRole('button', {
      name: new RegExp(`^(?!Actions for).*${escapeRegExp(videoTitle)}(?:\\s*(?:\\d+(?:h|d|w|mo|y) ago|just now))?$`),
    })
    await expect(row.getByRole('progressbar', { name: 'Watch progress' })).toBeVisible()
    await expect(row.getByText(TIME_LEFT)).toBeVisible()
  })

  test('sync changes nothing when up to date', async ({ page }) => {
    // Sync from the sidebar completes without an error dialog/alert, and
    // syncing an up-to-date playlist changes nothing: the same videos, still
    // downloaded, and nothing queued to download again.
    await openPlaylist(page)
    const videosUrl = `/api/playlists/${encodeURIComponent(seeded.id)}/videos`
    const videoState = async () =>
      (await (await page.request.get(videosUrl)).json()).map(({ id, status, filename }) => ({ id, status, filename }))
    const videosBeforeSync = await videoState()
    await syncItem(page, { section: 'Playlists', name: PLAYLIST_NAME })
    expect(await videoState()).toEqual(videosBeforeSync)
    const tasks = await (await page.request.get('/api/tasks')).json()
    expect(tasks.filter((task) => task.task_type === 'download_video')).toEqual([])
  })

  test('edits the check interval', async ({ page }) => {
    // Change how often the playlist is checked from its page header's "⋮"
    // menu; reopening the dialog shows the saved value (covers PATCH
    // /playlists/{id} with an interval).
    await openPlaylist(page)
    const editPlaylist = async () => {
      await page.getByRole('button', { name: 'Edit settings' }).click()
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

  test('runs its sync now from the Tasks view', async ({ page }) => {
    // Covers the Tasks view's "Run now".
    await page.goto('/tasks')
    await page.getByRole('tab', { name: /^Syncs/ }).click()
    const syncRow = page.locator('main li', { hasText: `Syncing playlist ${PLAYLIST_NAME}` })
    const ranNow = page.waitForResponse(
      (response) => response.url().endsWith('/reconcile') && response.request().method() === 'POST',
    )
    await syncRow.getByRole('button', { name: /^Run now: / }).click()
    expect((await ranNow).ok()).toBe(true)
    await expect(syncRow.getByRole('button', { name: /^Run now: / })).toBeEnabled()
    await expect(syncRow.getByText(/^Sync failed/)).toHaveCount(0)
  })

  test('rejects adding it twice', async ({ page }) => {
    // Adding the same playlist again is caught in the dialog, before
    // submission: the notice names what it was added as and the submit button
    // stays disabled.
    await page.goto('/')
    await openAddPlaylistDialog(page)
    const dialog = await fillPlaylist(page, { url: PLAYLIST_ID })
    await expect(destinationNotice(dialog)).toHaveText(`Already added as “${PLAYLIST_NAME}”`)
    await expect(dialog.getByRole('button', { name: /^Create Playlist/ })).toBeDisabled()
    await dialog.getByRole('button', { name: 'Close' }).click()
  })

  test('marks it watched while playing', async ({ page }) => {
    // Playing past the watched threshold marks the video watched without
    // interrupting playback: once the list shows it watched, the player is
    // still playing where it was, not rewound and paused.
    await openPlaylist(page)
    await assertVideoPlays(page)
    const player = page.locator('video')
    const markedWatched = page.waitForResponse(
      async (response) =>
        response.url().endsWith('/progress') &&
        response.request().method() === 'POST' &&
        (await response.json()).watched === true,
      { timeout: 30_000 },
    )
    // The player reports every 15 s while playing; at quarter speed the
    // short fixture clip is still playing then, past 90%.
    const nearEnd = await player.evaluate(async (el) => {
      el.playbackRate = 0.25
      el.currentTime = el.duration * 0.91
      await el.play()
      return el.currentTime
    })
    await markedWatched
    await expect(page.locator('main ul li').getByLabel('Watched')).toBeVisible({ timeout: 10_000 })
    expect(await player.evaluate((el) => el.paused)).toBe(false)
    expect(await player.evaluate((el) => el.currentTime)).toBeGreaterThan(nearEnd)
  })

  test('marks it all watched from its row menu', async ({ page }) => {
    // Covers POST /playlists/{id}/watched; with its only video watched, the
    // row shows no badge.
    await openPlaylist(page)
    await markItemWatched(page, { section: 'Playlists', name: PLAYLIST_NAME })
    await expect(unwatchedBadge(playlistRow(page))).toHaveCount(0)
  })
})

test('Downloads tab lists a running download', async ({ page }) => {
  // The slow fixture video takes about 8 s to download, long enough to be
  // seen running.
  const playlist = await addPlaylist(page.request, SLOW_PLAYLIST, { downloaded: false })

  await page.goto('/tasks')
  await page.getByRole('tab', { name: /^Downloads/ }).click()

  const download = page.locator('main li', { hasText: new RegExp(`^Downloading .+ in ${playlist.name}`) })
  await expect(download).toHaveCount(1, { timeout: 15_000 })
  // Still listed while it runs: the video isn't downloaded yet.
  const [video] = await (await page.request.get(`/api/playlists/${encodeURIComponent(playlist.id)}/videos`)).json()
  expect(video.status).not.toBe('DOWNLOADED')
})
