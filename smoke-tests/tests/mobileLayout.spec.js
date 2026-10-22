import { test, expect } from '../helpers/fixtures.js'
import { expectNoHorizontalOverflow } from '../helpers/layout.js'
import { deleteSeeded, seedDownloadedPlaylist } from '../helpers/seed.js'

const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'

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

test.fixme('add, edit-settings and delete dialogs fit the screen and submit @mobile', async () => {})

test.fixme('icon-only Tasks tabs and Run now work by accessible name @mobile', async () => {})
