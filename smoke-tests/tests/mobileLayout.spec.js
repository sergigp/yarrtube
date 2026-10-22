import { test, expect } from '../helpers/fixtures.js'
import { fillPlaylist, openFolderBrowser, openAdvancedOptions } from '../helpers/addDialog.js'
import { expectNoHorizontalOverflow, expectWithinViewport } from '../helpers/layout.js'
import { deleteSeeded, seedDownloadedPlaylist } from '../helpers/seed.js'
import { openDrawer } from '../helpers/sidebar.js'

const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'
const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
const PLAYLIST_NAME = process.env.SMOKE_PLAYLIST_NAME ?? 'test'

test.describe('with a downloaded playlist, a channel and a category', () => {
  test.use({ keepLibrary: true })

  let seeded
  let categoryId
  test.beforeAll(async ({ request }) => {
    seeded = await seedDownloadedPlaylist(request)
    // The channel's view renders whether or not its video has downloaded yet.
    const channel = await request.post('/api/channels', {
      data: {
        channel: CHANNEL_HANDLE,
        quality: 'low',
        video_limit: 1,
        path: 'channels/mobile-layout',
        reconcile_interval_minutes: 180,
      },
    })
    expect(channel.ok(), await channel.text()).toBeTruthy()
    const category = await request.post('/api/categories', { data: { name: 'Mobile layout' } })
    expect(category.ok(), await category.text()).toBeTruthy()
    categoryId = (await category.json()).id
    const members = await request.put(`/api/categories/${categoryId}/members`, {
      data: { channels: [CHANNEL_HANDLE], playlists: [seeded.id] },
    })
    expect(members.ok(), await members.text()).toBeTruthy()
  })

  test.afterAll(async ({ request }) => {
    await deleteSeeded(request, seeded)
  })

  test('no view scrolls horizontally @mobile', async ({ page }) => {
    const views = [
      { path: '/', shown: page.getByRole('heading', { name: 'Latest videos' }) },
      {
        path: `/playlists/${encodeURIComponent(seeded.id)}`,
        shown: page.getByRole('heading', { level: 3, name: seeded.video.title }),
      },
      {
        path: `/channels/${encodeURIComponent(CHANNEL_HANDLE)}`,
        shown: page.getByRole('heading', { level: 2 }),
      },
      {
        path: `/categories/${encodeURIComponent(categoryId)}`,
        shown: page.getByRole('heading', { name: 'Mobile layout' }),
      },
      { path: '/tasks', shown: page.getByRole('tab').first() },
    ]
    for (const { path, shown } of views) {
      await page.goto(path)
      await expect(shown).toBeVisible()
      await expectNoHorizontalOverflow(page)
    }
  })
})

test('add, edit-settings and delete dialogs fit the screen and submit @mobile', async ({ page }) => {
  await page.goto('/')

  await test.step('add a playlist with the folder browser open', async () => {
    await openDrawer(page)
    await page.getByRole('button', { name: 'Add playlist', exact: true }).tap()
    const dialog = await fillPlaylist(page, { url: PLAYLIST_ID })
    await openAdvancedOptions(dialog)
    await openFolderBrowser(dialog)
    const create = dialog.getByRole('button', { name: /^Create Playlist/ })
    await expectWithinViewport(page, dialog)
    // The dialog scrolls within itself; its submit button must be reachable.
    await create.scrollIntoViewIfNeeded()
    await expectWithinViewport(page, create)
    const created = page.waitForResponse(
      (response) => response.request().method() === 'POST' && response.url().endsWith('/api/playlists'),
    )
    await create.tap()
    expect((await created).status()).toBe(201)
    await expect(dialog).toBeHidden()
  })

  await page.goto(`/playlists/${encodeURIComponent(PLAYLIST_ID)}`)
  const moreActions = page.getByRole('button', { name: `More actions for ${PLAYLIST_NAME}` })

  await test.step('edit its settings', async () => {
    await moreActions.tap()
    await page.getByRole('menuitem', { name: 'Edit settings' }).tap()
    const dialog = page.getByRole('dialog', { name: `Edit ${PLAYLIST_NAME} settings` })
    const save = dialog.getByRole('button', { name: 'Save' })
    await dialog.getByLabel('Check for new videos every').fill('2')
    await expectWithinViewport(page, dialog)
    await expectWithinViewport(page, save)
    const saved = page.waitForResponse(
      (response) => response.request().method() === 'PATCH' && response.url().includes('/playlists/'),
    )
    await save.tap()
    expect((await saved).status()).toBe(200)
    await expect(dialog).toBeHidden()
  })

  await test.step('delete it', async () => {
    await moreActions.tap()
    await page.getByRole('menuitem', { name: 'Delete', exact: true }).tap()
    const dialog = page.getByRole('dialog', { name: `Delete "${PLAYLIST_NAME}"?` })
    const confirm = dialog.getByRole('button', { name: 'Delete', exact: true })
    await expectWithinViewport(page, dialog)
    await expectWithinViewport(page, confirm)
    const deleted = page.waitForResponse((response) => response.request().method() === 'DELETE')
    await confirm.tap()
    expect((await deleted).ok()).toBe(true)
    await expect(dialog).toBeHidden()
    await expect(page).toHaveURL(/\/$/)
  })
})

test.fixme('icon-only Tasks tabs and Run now work by accessible name @mobile', async () => {})
