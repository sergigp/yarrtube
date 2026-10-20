import { test, expect } from '../helpers/fixtures.js'
import { openAddChannelDialog, fillChannel } from '../helpers/addDialog.js'
import { sectionHeading, sectionRows } from '../helpers/sidebar.js'

const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'

test('category on mobile: add from the drawer, manage sources full screen, then add from the heading', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  await page.goto('/')

  // Scoped below the app root, as in mobile-sidebar.spec.js.
  const overlay = page.locator('#root div[aria-hidden="true"]')
  const dialog = page.getByRole('dialog')
  const addCategory = page.getByRole('button', { name: 'Add category', exact: true })

  await test.step('the add row closes the drawer before its dialog opens', async () => {
    await page.getByRole('button', { name: 'Open menu' }).click()
    await expect(overlay).toBeVisible()
    await expect(addCategory).toHaveText('Add category')
    await addCategory.click()
    await expect(overlay).toHaveCount(0)
    await expect(dialog).toContainText('Add category')
  })

  await test.step('creating it opens its manage sources dialog full screen', async () => {
    await dialog.getByLabel('Name').fill('Chess')
    await dialog.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page).toHaveURL(/\/categories\/[^?]+$/)
    await expect(dialog).toContainText('Manage sources in Chess')
    await expect(dialog).toContainText('No channels or playlists to add yet.')
    const box = await dialog.boundingBox()
    expect([box.x, box.y, box.width, box.height].map(Math.round)).toEqual([0, 0, 390, 844])
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click()
    await dialog.waitFor({ state: 'hidden' })
  })

  await test.step('with a category, the heading "+" adds the next one', async () => {
    await page.getByRole('button', { name: 'Open menu' }).click()
    await expect(sectionRows(page, 'Categories')).toHaveText(['Chess'])
    // The "+" is an icon: its name comes from its label, it has no text.
    await expect(addCategory).toHaveText('')
    await addCategory.click()
    await expect(overlay).toHaveCount(0)
    await expect(dialog).toContainText('Add category')
  })
})

test('category lifecycle: create, manage sources, rename, assign from the add channel dialog, view, mark watched, delete', async ({ page }) => {
  await page.goto('/')

  const dialog = page.getByRole('dialog')
  const categoryRows = sectionRows(page, 'Categories')
  const summary = page.locator('main').getByText(/^\d+ sources? · /)
  const openManageSources = async (name) => {
    await page.getByRole('button', { name: `More actions for ${name}` }).click()
    await page.getByRole('menuitem', { name: 'Manage sources' }).click()
    await expect(dialog).toContainText(`Manage sources in ${name}`)
  }

  await test.step('create it from the sidebar, landing in its manage sources dialog', async () => {
    await page.getByRole('button', { name: 'Add category', exact: true }).click()
    await dialog.getByLabel('Name').fill('Chess')
    await dialog.getByRole('button', { name: 'Create', exact: true }).click()
    await expect(page).toHaveURL(/\/categories\/[^?]+$/)
    await expect(dialog).toContainText('Manage sources in Chess')
    await expect(dialog.getByRole('searchbox', { name: 'Search channels and playlists' })).toBeFocused()
    await dialog.getByRole('button', { name: 'Cancel', exact: true }).click()
    await dialog.waitFor({ state: 'hidden' })
    await expect(categoryRows).toHaveText(['Chess'])
  })

  await test.step('rename it in place', async () => {
    await page.getByRole('button', { name: 'Rename', exact: true }).click()
    const name = page.getByRole('textbox', { name: 'Name', exact: true })
    await name.fill('Board games')
    await name.press('Enter')
    await expect(page.getByRole('heading', { name: 'Board games', exact: true })).toBeVisible()
    await expect(categoryRows).toHaveText([/^Board games/])
  })

  const channelName = await test.step('assign a new channel to it from the add channel dialog', async () => {
    await openAddChannelDialog(page)
    await fillChannel(page, { handle: CHANNEL_HANDLE, videoLimit: 1 })
    await dialog.getByRole('combobox', { name: 'Categories' }).fill('Board')
    await dialog.getByRole('option', { name: 'Board games' }).click()
    await expect(dialog.getByRole('list', { name: 'Chosen categories' })).toHaveText('Board games')
    await dialog.getByRole('button', { name: /^Create Channel/ }).click()
    await dialog.waitFor({ state: 'hidden' })
    const [{ name }] = await (await page.request.get('/api/channels')).json()
    await expect(summary).toHaveText(/^1 source · /)
    return name
  })

  await test.step('remove it and add it back with the chips', async () => {
    await openManageSources('Board games')
    const chip = dialog.getByRole('button', { name: channelName, exact: true })
    await expect(chip).toHaveAttribute('aria-pressed', 'true')
    await chip.click()
    await expect(dialog).toContainText('−1 removed')
    await dialog.getByRole('button', { name: 'Save', exact: true }).click()
    await dialog.waitFor({ state: 'hidden' })
    await expect(page.getByRole('region', { name: 'Nothing in Board games yet' })).toBeVisible()

    await page.getByRole('button', { name: 'Add channels and playlists' }).click()
    const search = dialog.getByRole('searchbox', { name: 'Search channels and playlists' })
    await search.fill(channelName)
    await search.press('Enter')
    await expect(dialog.getByRole('button', { name: channelName, exact: true })).toHaveAttribute(
      'aria-pressed',
      'true',
    )
    await expect(dialog).toContainText('+1 added')
    await dialog.getByRole('button', { name: 'Save', exact: true }).click()
    await dialog.waitFor({ state: 'hidden' })
    await expect(summary).toHaveText(/^1 source · /)
  })

  await test.step('wait for the channel video and mark it all watched', async () => {
    // The video card shows once the channel's download finishes.
    const cards = page.locator('main ul li')
    await expect(cards).toHaveCount(1, { timeout: 240_000 })
    // The sidebar refetches its badges on an interval, not when a download ends.
    await page.reload()
    await expect(categoryRows.getByLabel('1 unwatched')).toBeVisible()
    const marked = page.waitForResponse(
      (response) => response.url().endsWith('/watched') && response.request().method() === 'POST',
    )
    await page.getByRole('button', { name: 'More actions for Board games' }).click()
    await page.getByRole('menuitem', { name: 'Mark all watched' }).click()
    expect((await marked).status()).toBe(204)
    await expect(cards.getByRole('img', { name: 'Watched' })).toBeVisible()
    await expect(categoryRows.getByLabel(/ unwatched$/)).toHaveCount(0)
  })

  await test.step('let the channel finish fetching its subtitles', async () => {
    // Deleting the channel while it still writes its folder would leave files
    // behind for the next spec tracking the same channel.
    await expect
      .poll(
        async () => {
          const response = await page.request.get(
            `/api/channels/${encodeURIComponent(CHANNEL_HANDLE)}/videos`,
          )
          return (await response.json()).map((video) => video.subtitle_language !== null)
        },
        { timeout: 60_000 },
      )
      .toEqual([true])
  })

  await test.step('fold it with its heading', async () => {
    await sectionHeading(page, 'Categories').click()
    await expect(categoryRows).toHaveCount(0)
    await sectionHeading(page, 'Categories').click()
    await expect(categoryRows).toHaveText([/^Board games/])
  })

  await test.step('delete it from its actions menu, keeping its channel', async () => {
    await page.getByRole('button', { name: 'More actions for Board games' }).click()
    await page.getByRole('menuitem', { name: 'Delete', exact: true }).click()
    await expect(dialog).toContainText(
      '"Board games" holds 1 channels and playlists. None of them, nor any video, will be deleted.',
    )
    await dialog.getByRole('button', { name: 'Delete', exact: true }).click()
    await dialog.waitFor({ state: 'hidden' })
    await expect(categoryRows).toHaveCount(0)
    await expect(page).toHaveURL(/\/$/)
    await expect(sectionRows(page, 'Channels')).toHaveCount(1)
  })
})
