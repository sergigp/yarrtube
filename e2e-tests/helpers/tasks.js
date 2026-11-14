import { expect } from '@playwright/test'

// Opens the Tasks view through the settings menu.
export async function openTasks(page) {
  await page.getByRole('banner').getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('menuitem', { name: 'Tasks', exact: true }).click()
}

// The Tasks view's row for the scheduled sync of the source named `name`,
// on its own row: the search leaves no other sync to batch it with.
export async function syncRow(page, name) {
  const syncs = page.getByRole('button', { name: /^Syncs/ })
  await syncs.click()
  await expect(syncs).toHaveAttribute('aria-pressed', 'true')
  await page.getByRole('searchbox', { name: 'Search tasks' }).fill(name)
  return page.locator('main li', { hasText: `Check ${name} for new videos` })
}

// Opens the Tasks view and checks the source's single sync, of the playlist
// or channel named `name`, is scheduled at a clock time and due `due`
// (e.g. /^in (29|30) min$/).
export async function expectPendingSync(page, { name, due }) {
  await openTasks(page)
  const row = await syncRow(page, name)
  await expect(row).toHaveCount(1)
  await expect(row.getByText(/^(Tomorrow |\w{3} )?\d{1,2}:\d{2}( [AP]M)?$/)).toBeVisible()
  await expect(row.getByText(due)).toBeVisible()
}
