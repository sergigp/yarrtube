import { expect } from '@playwright/test'

export function sectionRows(page, section) {
  return page.locator(`h3:text-is("${section}") ~ ul li`)
}

async function rowFor(page, section, name) {
  const rows = sectionRows(page, section)
  const named = rows.filter({ hasText: name })
  if ((await named.count()) > 0) {
    return named
  }
  // Channels display their YouTube-fetched name, not the handle callers
  // identify them by, so `name` never matches — only safe to fall back to
  // "the only row" when there's exactly one; otherwise which row is meant
  // is genuinely ambiguous.
  const count = await rows.count()
  if (count === 1) {
    return rows
  }
  throw new Error(`sidebar: no "${section}" row matching "${name}", and ${count} candidates present`)
}

export async function syncItem(page, { section, name }) {
  const row = await rowFor(page, section, name)
  const button = row.getByRole('button', { name: /^Sync/ })

  let alertMessage = null
  const onDialog = async (dialog) => {
    alertMessage = dialog.message()
    await dialog.dismiss()
  }
  page.on('dialog', onDialog)
  try {
    await button.click()
    await expect(button).toBeEnabled({ timeout: 15_000 })
  } finally {
    page.off('dialog', onDialog)
  }
  if (alertMessage) {
    throw new Error(`sync failed: ${alertMessage}`)
  }
}

export async function deleteItem(page, { section, name }) {
  const row = await rowFor(page, section, name)
  await row.getByRole('button', { name: /^Delete/ }).click()
  const dialog = page.getByRole('dialog')
  await dialog.getByRole('button', { name: 'Delete', exact: true }).click()
  await dialog.waitFor({ state: 'hidden' })
}
