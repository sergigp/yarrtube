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

// Row actions live in each row's "Actions for …" menu, rendered in a portal
// outside the row, so menu items are looked up on the page.
async function chooseRowAction(page, row, action) {
  await row.getByRole('button', { name: /^Actions for / }).click()
  await page.getByRole('menuitem', { name: action, exact: true }).click()
}

export async function syncItem(page, { section, name }) {
  const row = await rowFor(page, section, name)

  let alertMessage = null
  const onDialog = async (dialog) => {
    alertMessage = dialog.message()
    await dialog.dismiss()
  }
  page.on('dialog', onDialog)
  try {
    const synced = page.waitForResponse(
      (response) => response.url().endsWith('/reconcile') && response.request().method() === 'POST',
      { timeout: 15_000 },
    )
    await chooseRowAction(page, row, 'Sync')
    await synced
  } finally {
    page.off('dialog', onDialog)
  }
  if (alertMessage) {
    throw new Error(`sync failed: ${alertMessage}`)
  }
}

export async function deleteItem(page, { section, name }) {
  const row = await rowFor(page, section, name)
  await chooseRowAction(page, row, 'Delete')
  const dialog = page.getByRole('dialog')
  await dialog.getByRole('button', { name: 'Delete', exact: true }).click()
  await dialog.waitFor({ state: 'hidden' })
}

export function unwatchedBadge(row) {
  return row.getByLabel(/ unwatched$/)
}

export async function markItemWatched(page, { section, name }) {
  const row = await rowFor(page, section, name)

  let alertMessage = null
  const onDialog = async (dialog) => {
    alertMessage = dialog.message()
    await dialog.dismiss()
  }
  page.on('dialog', onDialog)
  try {
    const marked = page.waitForResponse(
      (response) => response.url().endsWith('/watched') && response.request().method() === 'POST',
    )
    await chooseRowAction(page, row, 'Mark all watched')
    await marked
  } finally {
    page.off('dialog', onDialog)
  }
  if (alertMessage) {
    throw new Error(`mark watched failed: ${alertMessage}`)
  }
}

export async function includeItemInHome(page, { section, name }) {
  const row = await rowFor(page, section, name)
  const updated = page.waitForResponse(
    (response) => response.request().method() === 'PATCH' && response.url().includes('/playlists/'),
  )
  await chooseRowAction(page, row, 'Include in home')
  expect((await updated).status()).toBe(200)
}
