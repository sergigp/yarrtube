import { expect } from '@playwright/test'

// Opens the Tasks view's "Syncs" tab and checks a pending sync labelled
// `label` (e.g. "Syncing playlist test") is due `due` (e.g. /^in (29|30)m$/).
// Other syncs with the same label may be listed too: a source deleted and
// added again by an earlier test leaves its old sync queued.
export async function expectPendingSync(page, { label, due }) {
  await page.getByRole('banner').getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('menuitem', { name: 'Tasks', exact: true }).click()
  await page.getByRole('tab', { name: /^Syncs/ }).click()
  const row = page
    .locator('main li', { hasText: label })
    .filter({ has: page.getByText(due) })
    .filter({ has: page.getByText('pending', { exact: true }) })
  await expect(row).toHaveCount(1)
}
