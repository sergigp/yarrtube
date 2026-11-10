import { test, expect } from '../helpers/fixtures.js'
import { BROKEN_PLAYLIST } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'
import { waitForVideoStatus } from '../helpers/video.js'

// The stand-in yt-dlp fails this video's download, 2 s into each attempt,
// with a transient reason, so the daemon schedules its next attempt.
const FAILURE_REASON = 'Unable to download video data: HTTP Error 503: Service Unavailable'

test('a failed download is shown retrying with its reason, and Retry now runs it again', async ({ page }) => {
  const playlist = await addPlaylist(page.request, BROKEN_PLAYLIST, { downloaded: false })
  const videosUrl = `/api/playlists/${encodeURIComponent(playlist.id)}/videos`
  const downloadStep = async () => (await (await page.request.get(videosUrl)).json())[0].steps.download

  await test.step('after the first failure the video is retrying with its next attempt and reason', async () => {
    await page.goto(`/playlists/${encodeURIComponent(playlist.id)}`)
    await waitForVideoStatus(page, { status: 'RETRYING', timeoutMs: 30_000 })
    await expect(
      page.locator('main').getByText(`Download: retrying, next attempt in 10 minutes · ${FAILURE_REASON}`),
    ).toBeVisible()
  })

  await test.step('its download is listed pending for that attempt with the reason', async () => {
    await page.goto('/tasks')
    await page.getByRole('tab', { name: /^Downloads/ }).click()
    const download = page.locator('main li', { hasText: new RegExp(`^Downloading .+ in ${playlist.name}`) })
    await expect(download.getByText(FAILURE_REASON).first()).toBeVisible({ timeout: 15_000 })
    await expect(download.getByText('pending', { exact: true })).toBeVisible()
  })

  await test.step('Retry now runs the download again at once', async () => {
    const before = await downloadStep()
    await page.goto(`/playlists/${encodeURIComponent(playlist.id)}`)
    const retried = page.waitForResponse(
      (response) => response.request().method() === 'POST' && response.url().endsWith('/retry'),
    )
    await page.locator('main').getByRole('button', { name: 'Retry now' }).click()
    expect((await retried).status()).toBe(204)
    // It fails again: its failed attempts start over, its next attempt later.
    await expect
      .poll(async () => (await downloadStep()).next_attempt_at > before.next_attempt_at, { timeout: 30_000 })
      .toBe(true)
    expect(await downloadStep()).toEqual(
      expect.objectContaining({ status: 'RETRYING', failed_attempts: 1, failure: FAILURE_REASON }),
    )
  })
})
