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

async function submitAndSettle(dialog) {
  await Promise.race([
    dialog.waitFor({ state: 'hidden' }),
    dialogErrorText(dialog.page()).waitFor({ state: 'visible' }),
  ])
}

export async function submitPlaylist(page, { url, name, path }) {
  const dialog = page.getByRole('dialog')
  await dialog.getByRole('tab', { name: 'Playlist' }).click()
  await dialog.getByLabel('Playlist ID or URL').fill(url)
  await dialog.getByLabel('Name', { exact: true }).fill(name)
  if (path) {
    await openAdvancedOptions(dialog)
    await dialog.getByLabel('Path').fill(path)
  }
  await dialog.getByRole('button', { name: /^Create Playlist/ }).click()
  await submitAndSettle(dialog)
}

export async function submitChannel(page, { handle, videoLimit, path }) {
  const dialog = page.getByRole('dialog')
  await dialog.getByRole('tab', { name: 'Channel' }).click()
  await dialog.getByLabel('Channel Handle or URL').fill(handle)
  if (path || videoLimit !== undefined) {
    await openAdvancedOptions(dialog)
    if (path) {
      await dialog.getByLabel('Path').fill(path)
    }
    if (videoLimit !== undefined) {
      await dialog.getByLabel('Video Limit').fill(String(videoLimit))
    }
  }
  await dialog.getByRole('button', { name: /^Create Channel/ }).click()
  await submitAndSettle(dialog)
}

export function dialogErrorText(page) {
  return page.getByRole('dialog').locator('p.text-destructive')
}
