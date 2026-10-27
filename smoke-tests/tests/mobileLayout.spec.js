import { test, expect } from '../helpers/fixtures.js'
import { fillPlaylist, openFolderBrowser, openAdvancedOptions } from '../helpers/addDialog.js'
import { expectNoHorizontalOverflow, expectWithinViewport } from '../helpers/layout.js'
import { addChannel, addPlaylist } from '../helpers/seed.js'
import { CHANNEL, PLAYLIST } from '../helpers/fakeYoutube.js'
import { openDrawer } from '../helpers/sidebar.js'

const CHANNEL_HANDLE = CHANNEL.handle
const PLAYLIST_ID = PLAYLIST.id
const PLAYLIST_NAME = PLAYLIST.title

test.describe('with a downloaded playlist, a channel and a category', () => {
  let seeded
  let categoryId
  test.beforeEach(async ({ request }) => {
    const playlist = await addPlaylist(request, PLAYLIST)
    seeded = { ...playlist, video: playlist.videos[0] }
    await addChannel(request, CHANNEL, { videoLimit: 1 })
    const category = await request.post('/api/categories', { data: { name: 'Mobile layout' } })
    expect(category.ok(), await category.text()).toBeTruthy()
    categoryId = (await category.json()).id
    const members = await request.put(`/api/categories/${categoryId}/members`, {
      data: { channels: [CHANNEL_HANDLE], playlists: [seeded.id] },
    })
    expect(members.ok(), await members.text()).toBeTruthy()
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
        shown: page.getByRole('heading', { level: 2, name: 'Mobile layout', exact: true }),
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

test('icon-only Tasks tabs and Run now work by accessible name @mobile', async ({ page }) => {
  // A tracked playlist brings a sync to run; it is added through the API, with
  // the Add dialog's body, since only the Tasks view's controls are under test.
  const created = await page.request.post('/api/playlists', {
    data: {
      playlist: PLAYLIST_ID,
      path: 'playlists/mobile-tasks',
      quality: 'low',
      exclude_from_home: false,
      reconcile_interval_minutes: 180,
    },
  })
  expect(created.ok(), await created.text()).toBeTruthy()
  const reconciled = (page) =>
    page.waitForResponse(
      (response) => response.url().endsWith('/reconcile') && response.request().method() === 'POST',
    )

  await test.step("the header's Sync", async () => {
    await page.goto(`/playlists/${encodeURIComponent(PLAYLIST_ID)}`)
    const sync = page.getByRole('button', { name: 'Sync', exact: true })
    // Only its icon shows on a phone.
    await expect(sync).toHaveText('', { useInnerText: true })
    const synced = reconciled(page)
    await sync.tap()
    expect((await synced).ok()).toBe(true)
  })

  await test.step('the Syncs tab and its Run now', async () => {
    await page.goto('/tasks')
    const syncsTab = page.getByRole('tab', { name: /^Syncs/ })
    await syncsTab.tap()
    await expect(syncsTab).toHaveAttribute('aria-selected', 'true')

    const syncRow = page.locator('main li', { hasText: `Syncing playlist ${PLAYLIST_NAME}` })
    const runNow = syncRow.getByRole('button', { name: /^Run now: / })
    // The header's Sync may still be running it; Run now shows once it's pending again.
    await expect(runNow).toBeVisible({ timeout: 60_000 })
    await expect(runNow).toHaveText('', { useInnerText: true })
    const ranNow = reconciled(page)
    await runNow.tap()
    expect((await ranNow).ok()).toBe(true)
    await expect(syncRow.getByText(/^Sync failed/)).toHaveCount(0)
  })
})
