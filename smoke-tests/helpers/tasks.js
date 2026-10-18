import { expect } from '@playwright/test'

// Opens the Tasks view's "Syncs" tab and checks the source's single sync,
// labelled `label` (e.g. "Syncing playlist test"), is pending and due `due`
// (e.g. /^in (29|30)m$/).
export async function expectPendingSync(page, { label, due }) {
  await page.getByRole('banner').getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('menuitem', { name: 'Tasks', exact: true }).click()
  await page.getByRole('tab', { name: /^Syncs/ }).click()
  const rows = page.locator('main li', { hasText: label })
  await expect(rows).toHaveCount(1)
  await expect(rows.getByText('pending', { exact: true })).toBeVisible()
  await expect(rows.getByText(due)).toBeVisible()
}
