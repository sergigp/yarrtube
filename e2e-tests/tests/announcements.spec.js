import { test, expect } from '../helpers/fixtures.js'

// The live file on `main` changes over time, so every test stubs it.
const ANNOUNCEMENTS_URL =
  'https://raw.githubusercontent.com/sergigp/yarrtube/main/announcements/announcements.json'

test('announcements bar shows, links, dismisses and remembers dismissals', async ({ page }) => {
  await page.route(ANNOUNCEMENTS_URL, (route) =>
    route.fulfill({
      json: [
        { id: 'e2e-b', text: 'Second [docs](https://example.com/docs)' },
        { id: 'e2e-a', text: 'First' },
      ],
    }),
  )
  await page.goto('/')

  const bar = page.getByRole('region', { name: 'Announcement' })
  await expect(bar).toHaveText('Second docs')
  const link = bar.getByRole('link', { name: 'docs' })
  await expect(link).toHaveAttribute('href', 'https://example.com/docs')
  await expect(link).toHaveAttribute('target', '_blank')

  await bar.getByRole('button', { name: 'Dismiss announcement' }).click()
  await expect(bar).toHaveText('First')

  await bar.getByRole('button', { name: 'Dismiss announcement' }).click()
  await expect(bar).toHaveCount(0)

  // Wait for the announcements themselves, so the bar's absence isn't just
  // "not fetched yet".
  const announcementsLoaded = page.waitForResponse(ANNOUNCEMENTS_URL)
  await page.reload()
  await announcementsLoaded
  await expect(page.getByRole('banner')).toBeVisible()
  await expect(bar).toHaveCount(0)
})

test('announcements bar stays hidden when the fetch fails', async ({ page }) => {
  await page.route(ANNOUNCEMENTS_URL, (route) => route.abort())
  const announcementsFailed = page.waitForEvent('requestfailed', (request) => request.url() === ANNOUNCEMENTS_URL)
  await page.goto('/')
  await announcementsFailed

  await expect(page.getByRole('banner')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Add channel', exact: true })).toBeVisible()
  await expect(page.getByRole('region', { name: 'Announcement' })).toHaveCount(0)
})
