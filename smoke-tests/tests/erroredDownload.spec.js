import { test, expect } from '../helpers/fixtures.js'
import { BROKEN_PLAYLIST } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'
import { waitForVideoStatus } from '../helpers/video.js'

// The stand-in yt-dlp fails this video's download, 2 s into each attempt,
// with a transient reason, so the daemon retries it (right away in the smoke
// container) until its attempts run out.
const FAILURE_REASON = 'Unable to download video data: HTTP Error 503: Service Unavailable'

test('a download that keeps failing is shown as errored with its reason', async ({ page }) => {
  const playlist = await addPlaylist(page.request, BROKEN_PLAYLIST, { downloaded: false })
  const [video] = BROKEN_PLAYLIST.videos

  await test.step('its download is listed with the reason while retried', async () => {
    await page.goto('/tasks')
    await page.getByRole('tab', { name: /^Downloads/ }).click()
    const download = page.locator('main li', { hasText: new RegExp(`^Downloading .+ in ${playlist.name}`) })
    await expect(download.getByText(FAILURE_REASON).first()).toBeVisible({ timeout: 15_000 })
    await expect(download.getByText(/^\d+ retries$/)).toBeVisible()
  })

  await test.step('once the retries run out, the video is errored', async () => {
    await page.goto(`/playlists/${encodeURIComponent(playlist.id)}`)
    await waitForVideoStatus(page, { status: 'ERRORED', timeoutMs: 30_000 })
    const videos = await (await page.request.get(`/api/playlists/${encodeURIComponent(playlist.id)}/videos`)).json()
    expect(videos.map(({ id, status }) => ({ id, status }))).toEqual([{ id: video, status: 'ERRORED' }])
  })
})
