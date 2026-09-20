import { expect } from '@playwright/test'

async function rowFor(page, section, name) {
  const rows = page.locator(`h3:text-is("${section}") ~ ul li`)
  const named = rows.filter({ hasText: name })
  return (await named.count()) > 0 ? named : rows
}

export async function syncItem(page, { section, name }) {
  const row = await rowFor(page, section, name)
  const button = row.getByRole('button', { name: /^Sync/ })
  await button.click()
  await expect(button).toBeEnabled({ timeout: 15_000 })
}

export async function deleteItem(page, { section, name }) {
  const row = await rowFor(page, section, name)
  await row.getByRole('button', { name: /^Delete/ }).click()
  const dialog = page.getByRole('dialog')
  await dialog.getByRole('button', { name: 'Delete', exact: true }).click()
  await dialog.waitFor({ state: 'hidden' })
}
