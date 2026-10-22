import { expect } from '@playwright/test'

// Each section is a region named after its heading, and its entries a list
// named the same. While folded the heading's name also carries the section's
// unwatched total ("Channels 3 unwatched").
function sectionRegion(page, section) {
  return page.getByRole('region', { name: new RegExp(`^${section}( \\d+ unwatched)?$`) })
}

export function sectionList(page, section) {
  return page.getByRole('list', { name: section, exact: true })
}

export function sectionRows(page, section) {
  return sectionList(page, section).getByRole('listitem')
}

export function sectionEmptyMessage(page, section) {
  return sectionRegion(page, section).getByRole('paragraph')
}

// The heading toggle that folds a section; while folded its name also
// carries the section's unwatched total ("Channels 3 unwatched").
export function sectionHeading(page, section) {
  return page.getByRole('button', { name: new RegExp(`^${section}( \\d+ unwatched)?$`) })
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
    expect((await synced).ok(), 'sync response').toBe(true)
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
    expect((await marked).ok(), 'mark watched response').toBe(true)
  } finally {
    page.off('dialog', onDialog)
  }
  if (alertMessage) {
    throw new Error(`mark watched failed: ${alertMessage}`)
  }
}

/** Hides or shows a playlist row's unwatched badge through its row menu. */
export async function setItemUnwatchedCountHidden(page, { section, name, hidden }) {
  const row = await rowFor(page, section, name)
  const updated = page.waitForResponse(
    (response) => response.request().method() === 'PATCH' && response.url().includes('/playlists/'),
  )
  await chooseRowAction(page, row, hidden ? 'Hide unwatched count' : 'Show unwatched count')
  expect((await updated).status()).toBe(200)
}

export async function includeItemInHome(page, { section, name }) {
  const row = await rowFor(page, section, name)
  const updated = page.waitForResponse(
    (response) => response.request().method() === 'PATCH' && response.url().includes('/playlists/'),
  )
  await chooseRowAction(page, row, 'Include in home')
  expect((await updated).status()).toBe(200)
}

// The mobile drawer is the sidebar (`#app-sidebar`) slid into view; the
// header's menu toggle controls it and reports its state in `aria-expanded`.
function drawerToggle(page) {
  return page.locator('button[aria-controls="app-sidebar"]')
}

function drawer(page) {
  return page.locator('#app-sidebar')
}

export async function openDrawer(page) {
  await drawerToggle(page).tap()
  await expectDrawerOpen(page)
}

export async function closeDrawer(page) {
  await drawerToggle(page).tap()
  await expectDrawerClosed(page)
}

export async function expectDrawerOpen(page) {
  await expect(drawerToggle(page)).toHaveAttribute('aria-expanded', 'true')
  await expect(drawer(page)).toBeInViewport()
}

export async function expectDrawerClosed(page) {
  await expect(drawerToggle(page)).toHaveAttribute('aria-expanded', 'false')
  await expect(drawer(page)).not.toBeInViewport()
}
