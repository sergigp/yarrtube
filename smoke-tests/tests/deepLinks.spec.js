import { test, expect } from '../helpers/fixtures.js'
import { slugOf } from '../helpers/addDialog.js'

const PLAYLIST_ID = process.env.SMOKE_PLAYLIST_ID
const PLAYLIST_NAME = process.env.SMOKE_PLAYLIST_NAME ?? 'test'
const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'

// The untracked playlist's videos are refused, which the view reports.
test.use({
  expectedApiErrors: [{ method: 'GET', path: '/api/playlists/PLyarrtubeNotTracked/videos', status: 400 }],
})

test('reloading each route shows its view; unknown playlist says Playlist not found.', async ({ page }) => {
  // Each view only needs its source to be tracked, so they are added through
  // the API, with the Add dialogs' bodies, and their downloads aren't waited on.
  const playlist = await page.request.post('/api/playlists', {
    data: {
      playlist: PLAYLIST_ID,
      path: `playlists/${slugOf(PLAYLIST_NAME)}`,
      quality: 'low',
      exclude_from_home: false,
      reconcile_interval_minutes: 180,
    },
  })
  expect(playlist.ok(), await playlist.text()).toBeTruthy()
  const channel = await page.request.post('/api/channels', {
    data: {
      channel: CHANNEL_HANDLE,
      quality: 'low',
      video_limit: 1,
      path: `channels/${slugOf(CHANNEL_HANDLE)}`,
      reconcile_interval_minutes: 180,
    },
  })
  expect(channel.ok(), await channel.text()).toBeTruthy()
  const { name: channelName } = await channel.json()
  const category = await page.request.post('/api/categories', { data: { name: 'Deep links' } })
  expect(category.ok(), await category.text()).toBeTruthy()
  const { id: categoryId } = await category.json()

  const main = page.locator('main')
  const views = [
    { path: '/', shown: main.getByText(/^(No videos synced yet\.|Latest videos)$/).first() },
    {
      path: `/playlists/${encodeURIComponent(PLAYLIST_ID)}`,
      shown: main.getByRole('heading', { level: 2, name: PLAYLIST_NAME }),
    },
    {
      path: `/channels/${encodeURIComponent(CHANNEL_HANDLE)}`,
      shown: main.getByRole('heading', { level: 2, name: channelName }),
    },
    {
      path: `/categories/${encodeURIComponent(categoryId)}`,
      shown: main.getByRole('heading', { level: 2, name: 'Deep links', exact: true }),
    },
    { path: '/tasks', shown: main.getByRole('tab', { name: /^Syncs/ }) },
  ]
  for (const { path, shown } of views) {
    await test.step(path, async () => {
      await page.goto(path)
      await expect(shown).toBeVisible()
      await page.reload()
      await expect(shown).toBeVisible()
    })
  }

  await test.step('an untracked playlist', async () => {
    await page.goto('/playlists/PLyarrtubeNotTracked')
    await expect(main.getByText('Playlist not found.', { exact: true })).toBeVisible()
  })
})
