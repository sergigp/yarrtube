import { test, expect } from '../helpers/fixtures.js'
import { addChannel, addPlaylist } from '../helpers/seed.js'
import { CHANNEL, PLAYLIST } from '../helpers/fakeYoutube.js'

/** Creates a category named `name` holding `members`, returning its id. */
async function addCategory(request, name, members) {
  const created = await request.post('/api/categories', { data: { name } })
  expect(created.ok(), await created.text()).toBeTruthy()
  const { id } = await created.json()
  const set = await request.put(`/api/categories/${id}/members`, { data: members })
  expect(set.ok(), await set.text()).toBeTruthy()
  return id
}

/** Opens the "Edit categories…" dialog of the video titled `title` from its "⋮". */
async function openEditCategories(page, title) {
  await page.getByRole('button', { name: `Actions for ${title}`, exact: true }).first().click()
  await page.getByRole('menuitem', { name: 'Edit categories…' }).click()
  const dialog = page.getByRole('dialog', { name: title })
  await expect(dialog).toBeVisible()
  return dialog
}

test.describe('with a channel in "Chess" and a "Sicilian" category', () => {
  let channel
  let playlist
  let sicilianId
  test.beforeEach(async ({ request }) => {
    channel = await addChannel(request, CHANNEL, { videoLimit: 1 })
    playlist = await addPlaylist(request, PLAYLIST)
    await addCategory(request, 'Chess', { channels: [channel.handle], playlists: [] })
    // A member, so the category page lists videos rather than its empty state.
    sicilianId = await addCategory(request, 'Sicilian', { channels: [], playlists: [playlist.id] })
  })

  test("edit a video's categories by hand: its chip and the category page follow", async ({
    page,
  }) => {
    const [{ title }] = channel.videos
    const sicilianPage = `/categories/${encodeURIComponent(sicilianId)}`
    const card = (title) => page.locator('main li').filter({ hasText: title }).first()

    await test.step('the channel\'s category is locked, the rest can be clicked', async () => {
      await page.goto(`/channels/${encodeURIComponent(channel.handle)}`)
      const dialog = await openEditCategories(page, title)
      const chess = dialog.getByRole('button', { name: `Chess, from ${channel.name}` })
      await expect(chess).toBeDisabled()
      await expect(chess).toHaveAttribute('aria-pressed', 'true')
      await expect(dialog.getByText(`From ${channel.name}`)).toBeVisible()
      await expect(dialog.getByRole('button', { name: 'Sicilian' })).toHaveAttribute(
        'aria-pressed',
        'false',
      )
    })

    await test.step('adding "Sicilian" by hand fills its chip', async () => {
      const dialog = page.getByRole('dialog', { name: title })
      await dialog.getByRole('button', { name: 'Sicilian' }).click()
      await expect(dialog.getByRole('button', { name: 'Sicilian' })).toHaveAttribute(
        'aria-pressed',
        'true',
      )
      await page.keyboard.press('Escape')
      await expect(dialog).toBeHidden()
    })

    await test.step('the video shows in "Sicilian"', async () => {
      await page.goto(sicilianPage)
      await expect(card(title)).toBeVisible()
    })

    await test.step('removing it by hand takes the video out of "Sicilian"', async () => {
      await page.goto(`/channels/${encodeURIComponent(channel.handle)}`)
      const dialog = await openEditCategories(page, title)
      await dialog.getByRole('button', { name: 'Sicilian' }).click()
      await expect(dialog.getByRole('button', { name: 'Sicilian' })).toHaveAttribute(
        'aria-pressed',
        'false',
      )
      await page.keyboard.press('Escape')
      await page.goto(sicilianPage)
      await expect(card(playlist.videos[0].title)).toBeVisible()
      await expect(page.locator('main li').filter({ hasText: title })).toHaveCount(0)
    })
  })

  test("re-run a video's AI categorization from its dialog", async ({ page, request, daemon }) => {
    const preferences = await request.patch('/api/preferences', {
      data: { ai_enabled: true, ollama_url: daemon.ollamaUrl, categorization_enabled: true },
    })
    expect(preferences.ok(), await preferences.text()).toBeTruthy()
    const [{ title }] = channel.videos
    await page.goto(`/channels/${encodeURIComponent(channel.handle)}`)
    const dialog = await openEditCategories(page, title)

    const deleted = page.waitForResponse(
      (response) =>
        response.request().method() === 'DELETE' && /\/api\/videos\/[^/]+\/ai-verdicts$/.test(response.url()),
    )
    await dialog.getByRole('button', { name: 'Re-run AI' }).click()

    expect((await deleted).status()).toBe(204)
  })

  test("clear a channel's and a playlist's AI categories from their menus", async ({ page }) => {
    for (const [path, name] of [
      [`/channels/${encodeURIComponent(channel.handle)}`, channel.name],
      [`/playlists/${encodeURIComponent(playlist.id)}`, playlist.name],
    ]) {
      await test.step(`clear ${name}`, async () => {
        await page.goto(path)
        await page.locator('main').getByRole('button', { name: `More actions for ${name}` }).click()
        await page.getByRole('menuitem', { name: 'Clear AI categories' }).click()
        const confirmation = page.getByRole('dialog', { name: `Clear AI categories of "${name}"?` })
        const deleted = page.waitForResponse(
          (response) =>
            response.request().method() === 'DELETE' && response.url().endsWith('/ai-verdicts'),
        )
        await confirmation.getByRole('button', { name: 'Clear', exact: true }).click()
        expect((await deleted).status()).toBe(204)
        await expect(confirmation).toBeHidden()
      })
    }
  })
})
