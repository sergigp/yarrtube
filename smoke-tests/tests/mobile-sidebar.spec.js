import { test, expect } from '../helpers/fixtures.js'
import { slugOf } from '../helpers/addDialog.js'
import { PLAYLIST } from '../helpers/fakeYoutube.js'
import {
  closeDrawer,
  expectDrawerClosed,
  openDrawer,
  sectionList,
} from '../helpers/sidebar.js'

const PLAYLIST_ID = PLAYLIST.id
const PLAYLIST_NAME = PLAYLIST.title

// The dimmed backdrop behind the open drawer. Scoped below the app root:
// while a dialog is open, Radix marks the root itself `aria-hidden`, and the
// dialog's own overlay is portalled to body.
function backdrop(page) {
  return page.locator('#root div[aria-hidden="true"]')
}

test('mobile sidebar opens and closes via toggle and backdrop @mobile', async ({ page }) => {
  await page.goto('/')
  await expectDrawerClosed(page)

  await openDrawer(page)
  await closeDrawer(page)

  await openDrawer(page)
  // Tap a point clear of the drawer itself (at most 85% of the width,
  // left-aligned) rather than the backdrop's center, which the
  // higher-stacked drawer covers.
  const { width } = page.viewportSize()
  await backdrop(page).tap({ position: { x: width - 20, y: 20 } })
  await expectDrawerClosed(page)
})

test('mobile sidebar stays closable when the announcement appears after it opened @mobile', async ({ page }) => {
  // Hold the announcements response until the sidebar is open, so the bar
  // appears above the header afterwards and pushes it down.
  let releaseAnnouncements
  const announcementsHeld = new Promise((resolve) => (releaseAnnouncements = resolve))
  await page.route('**/announcements/announcements.json', async (route) => {
    await announcementsHeld
    await route.fulfill({
      json: [{ id: 'smoke-late-banner', text: 'A late announcement' }],
    })
  })
  await page.goto('/')

  await openDrawer(page)
  releaseAnnouncements()
  await expect(page.getByRole('region', { name: 'Announcement' })).toBeVisible()

  await closeDrawer(page)
})

test('it should close the drawer and open the add channel dialog @mobile', async ({ page }) => {
  await page.goto('/')

  await openDrawer(page)
  await page.getByRole('button', { name: 'Add channel', exact: true }).tap()

  await expectDrawerClosed(page)
  await expect(page.getByRole('dialog', { name: 'Add channel' })).toBeVisible()
})

test('drawer closes on Escape @mobile', async ({ page }) => {
  await page.goto('/')

  await openDrawer(page)
  await page.keyboard.press('Escape')

  await expectDrawerClosed(page)
})

// Only the views it leads to are under test, so the playlist is added
// through the API, with the Add dialog's body, and isn't waited on.
async function addPlaylist(request) {
  const created = await request.post('/api/playlists', {
    data: {
      playlist: PLAYLIST_ID,
      path: `playlists/${slugOf(PLAYLIST_NAME)}`,
      quality: 'low',
      exclude_from_home: false,
      reconcile_interval_minutes: 180,
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
}

test('tapping a drawer entry navigates and closes it @mobile', async ({ page }) => {
  await addPlaylist(page.request)
  await page.goto('/')

  await openDrawer(page)
  await sectionList(page, 'Playlists').getByRole('link', { name: PLAYLIST_NAME }).tap()

  await expect(page).toHaveURL(`/playlists/${encodeURIComponent(PLAYLIST_ID)}`)
  await expect(page.getByRole('heading', { level: 2, name: PLAYLIST_NAME })).toBeVisible()
  await expectDrawerClosed(page)
})

test('dismissing the drawer keeps the scroll position and the page behind stays still @mobile', async ({ page }) => {
  await addPlaylist(page.request)
  await page.goto(`/playlists/${encodeURIComponent(PLAYLIST_ID)}`)
  // Expanded details make the page tall enough to scroll on a phone.
  await page.getByRole('button', { name: 'More', exact: true }).tap()
  await expect(page.getByRole('link', { name: 'Open on YouTube' })).toBeVisible()
  const scrollY = () => page.evaluate(() => window.scrollY)

  // Mobile WebKit has no mouse wheel, so the page is scrolled from the
  // keyboard, which, unlike a scripted scroll, honours the scroll lock.
  await page.keyboard.press('PageDown')
  // PageDown scrolls smoothly: read the position once it has stopped moving.
  let before = -1
  await expect
    .poll(
      async () => {
        const now = await scrollY()
        const settled = now > 0 && now === before
        before = now
        return settled
      },
      { intervals: [250] },
    )
    .toBe(true)

  await openDrawer(page)
  await page.keyboard.press('PageDown')
  // Give a scroll that shouldn't happen the time it would take.
  await page.waitForTimeout(500)
  expect(await scrollY()).toBe(before)

  const { width } = page.viewportSize()
  await backdrop(page).tap({ position: { x: width - 20, y: 20 } })
  await expectDrawerClosed(page)
  expect(await scrollY()).toBe(before)
})
