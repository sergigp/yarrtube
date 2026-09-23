export async function openAddDialog(page) {
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await page.getByRole('dialog').waitFor({ state: 'visible' })
}

async function openAdvancedOptions(dialog) {
  const toggle = dialog.getByRole('button', { name: /Advanced options/ })
  if ((await toggle.getAttribute('aria-expanded')) !== 'true') {
    await toggle.click()
  }
}

export async function openFolderBrowser(dialog) {
  const toggle = dialog.getByRole('button', { name: /^(Change|Done)$/ })
  if ((await toggle.getAttribute('aria-expanded')) !== 'true') {
    await toggle.click()
  }
}

function escapeForRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/**
 * The browser's entry for `name`. An occupied entry's accessible name carries
 * a trailing "in use by …", so this anchors on the start rather than matching
 * the whole string.
 */
export function folderEntry(dialog, name) {
  return dialog
    .getByRole('listitem')
    .getByRole('button', { name: new RegExp(`^${escapeForRegExp(name)}(\\s|$)`) })
}

export function breadcrumbLinks(dialog) {
  return dialog.getByRole('navigation', { name: 'Folder path' }).getByRole('button')
}

export function destinationPreview(dialog) {
  return dialog.getByTestId('destination-path')
}

async function isPresent(locator, timeout = 3000) {
  try {
    await locator.first().waitFor({ state: 'visible', timeout })
    return true
  } catch {
    return false
  }
}

/**
 * Walks the parent browser to `parent` (a path relative to the videos root),
 * starting from the root so the walk does not depend on where the dialog
 * opened. A segment that is already on disk is descended into; one that is
 * not is named in the create-folder step, which stages it.
 */
export async function browseToParent(dialog, parent) {
  await openFolderBrowser(dialog)

  const crumbs = breadcrumbLinks(dialog)
  if ((await crumbs.count()) > 0) {
    await crumbs.first().click()
  }

  for (const segment of parent.split('/').filter(Boolean)) {
    const entry = folderEntry(dialog, segment)
    if (await isPresent(entry)) {
      await entry.first().click()
    } else {
      await dialog.getByRole('button', { name: 'New folder' }).click()
      await dialog.getByLabel('New folder name').fill(segment)
      await dialog.getByRole('button', { name: 'Use folder' }).click()
    }
  }
}

async function setLocation(dialog, { parent, folderName }) {
  if (parent !== undefined) {
    await browseToParent(dialog, parent)
  }
  if (folderName !== undefined) {
    await dialog.getByLabel('Folder name').fill(folderName)
  }
}

async function submitAndSettle(dialog) {
  await Promise.race([
    dialog.waitFor({ state: 'hidden' }),
    dialogErrorText(dialog.page()).waitFor({ state: 'visible' }),
  ])
}

export async function fillPlaylist(page, { url, name, parent, folderName }) {
  const dialog = page.getByRole('dialog')
  await dialog.getByRole('tab', { name: 'Playlist' }).click()
  await dialog.getByLabel('Playlist ID or URL').fill(url)
  await dialog.getByLabel('Name', { exact: true }).fill(name)
  await setLocation(dialog, { parent, folderName })
  return dialog
}

export async function submitPlaylist(page, { url, name, parent, folderName }) {
  const dialog = await fillPlaylist(page, { url, name, parent, folderName })
  await dialog.getByRole('button', { name: /^Create Playlist/ }).click()
  await submitAndSettle(dialog)
}

export async function fillChannel(page, { handle, videoLimit, parent, folderName }) {
  const dialog = page.getByRole('dialog')
  await dialog.getByRole('tab', { name: 'Channel' }).click()
  await dialog.getByLabel('Channel Handle or URL').fill(handle)
  await setLocation(dialog, { parent, folderName })
  if (videoLimit !== undefined) {
    await openAdvancedOptions(dialog)
    await dialog.getByLabel('Video Limit').fill(String(videoLimit))
  }
  return dialog
}

export async function submitChannel(page, { handle, videoLimit, parent, folderName }) {
  const dialog = await fillChannel(page, { handle, videoLimit, parent, folderName })
  await dialog.getByRole('button', { name: /^Create Channel/ }).click()
  await submitAndSettle(dialog)
}

export function dialogErrorText(page) {
  return page.getByRole('dialog').locator('p.text-destructive')
}
