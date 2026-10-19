import { test, expect } from '../helpers/fixtures.js'

// The live file on `main` changes over time, so every test stubs it.
const ANNOUNCEMENTS_URL =
  'https://raw.githubusercontent.com/sergigp/yarrtube/main/announcements/announcements.json'

test('announcements bar shows, links, dismisses and remembers dismissals', async ({ page }) => {
  await page.route(ANNOUNCEMENTS_URL, (route) =>
    route.fulfill({
      json: [
        { id: 'smoke-b', text: 'Second [docs](https://example.com/docs)' },
        { id: 'smoke-a', text: 'First' },
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

  await page.reload()
  // Wait for the app shell so the bar's absence isn't just "not rendered yet".
  await expect(page.getByRole('banner')).toBeVisible()
  await page.waitForLoadState('networkidle')
  await expect(bar).toHaveCount(0)
})

test('announcements bar stays hidden when the fetch fails', async ({ page }) => {
  await page.route(ANNOUNCEMENTS_URL, (route) => route.abort())
  await page.goto('/')

  await expect(page.getByRole('banner')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Add channel', exact: true })).toBeVisible()
  await page.waitForLoadState('networkidle')
  await expect(page.getByRole('region', { name: 'Announcement' })).toHaveCount(0)
})
