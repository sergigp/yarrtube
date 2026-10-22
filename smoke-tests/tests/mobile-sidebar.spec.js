import { test, expect } from '../helpers/fixtures.js'
import { closeDrawer, expectDrawerClosed, expectDrawerOpen, openDrawer } from '../helpers/sidebar.js'

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

test.fixme('tapping a drawer entry navigates and closes it @mobile', async () => {})

test.fixme('dismissing the drawer keeps the scroll position and the page behind stays still @mobile', async () => {})
