import { test, expect } from '../helpers/fixtures.js'
import { openAddChannelDialog, fillChannel } from '../helpers/addDialog.js'
import { sectionHeading, sectionRows } from '../helpers/sidebar.js'

const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'

test('category lifecycle: create, rename, assign from the add channel dialog, manage members, view, mark watched, delete', async ({ page }) => {
  await page.goto('/')

  const dialog = page.getByRole('dialog')
  const categoryRows = sectionRows(page, 'Categories')

  await test.step('create it from the sidebar', async () => {
    await page.getByRole('button', { name: 'Add category', exact: true }).click()
    await dialog.getByLabel('Name').fill('Chess')
    await dialog.getByRole('button', { name: 'Create', exact: true }).click()
    await dialog.waitFor({ state: 'hidden' })
    await expect(page).toHaveURL(/\/categories\?selected=/)
    await expect(page.getByRole('searchbox', { name: 'Search channels and playlists to add' })).toBeFocused()
    await expect(categoryRows).toHaveText(['Chess'])
  })

  await test.step('rename it in place', async () => {
    const name = page.getByRole('textbox', { name: 'Category name' })
    await name.fill('Board games')
    await name.press('Enter')
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
    const members = page.getByRole('region', { name: 'In Board games' })
    await expect(members.getByText(name, { exact: true })).toBeVisible()
    return name
  })

  await test.step('remove it and add it back from the picker', async () => {
    const members = page.getByRole('region', { name: 'In Board games' })
    const picker = page.getByRole('group', { name: 'Add to Board games' })
    await members.getByRole('button', { name: `Remove ${channelName}` }).click()
    await expect(members.getByText(channelName, { exact: true })).toHaveCount(0)
    await page.getByRole('searchbox', { name: 'Search channels and playlists to add' }).fill(channelName)
    await picker.getByRole('checkbox', { name: channelName }).check()
    await picker.getByRole('button', { name: 'Add selected' }).click()
    await expect(members.getByText(channelName, { exact: true })).toBeVisible()
    await expect(picker.getByRole('checkbox', { name: channelName })).toHaveCount(0)
  })

  await test.step('open its view, wait for the channel video and mark it all watched', async () => {
    await categoryRows.getByRole('link', { name: /^Board games/ }).click()
    await expect(page).toHaveURL(/\/categories\/[^?]+$/)
    await expect(page.getByRole('heading', { name: 'Board games' })).toBeVisible()
    await expect(page.locator('main').getByText(channelName, { exact: true }).first()).toBeVisible()
    // The video card shows once the channel's download finishes.
    const cards = page.locator('main ul li')
    await expect(cards).toHaveCount(1, { timeout: 240_000 })
    // The sidebar refetches its badges on an interval, not when a download ends.
    await page.reload()
    await expect(categoryRows.getByLabel('1 unwatched')).toBeVisible()
    const marked = page.waitForResponse(
      (response) => response.url().endsWith('/watched') && response.request().method() === 'POST',
    )
    await page.getByRole('button', { name: 'Mark all watched' }).click()
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

  await test.step('delete it, keeping its channel', async () => {
    await categoryRows.getByRole('button', { name: 'Actions for Board games' }).click()
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

test('reaches the categories management view from the settings menu', async ({ page }) => {
  await page.goto('/')

  await page.getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('menuitem', { name: 'Categories' }).click()

  await expect(page).toHaveURL(/\/categories$/)
  await expect(page.getByText('No categories yet. Create one to group channels and playlists.')).toBeVisible()
})
