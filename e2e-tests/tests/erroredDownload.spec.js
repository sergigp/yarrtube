import { test, expect } from '../helpers/fixtures.js'
import { BROKEN_PLAYLIST } from '../helpers/fakeYoutube.js'
import { addPlaylist } from '../helpers/seed.js'
import { waitForVideoStatus } from '../helpers/video.js'

// The stand-in yt-dlp fails this video's download, 2 s into each attempt,
// with a transient reason, so the daemon schedules its next attempt.
const FAILURE_REASON = 'Unable to download video data: HTTP Error 503: Service Unavailable'

test('a failed download is shown retrying with its reason, Retry now runs it again, and Stop retrying fails it', async ({ page }) => {
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

  await test.step('its download needs attention in the Tasks view, with the reason', async () => {
    await page.goto('/tasks')
    const attention = page.getByRole('region', { name: 'Needs attention' })
    const card = attention.locator('li', { hasText: new RegExp(`Download .+ from ${playlist.name}`) })
    await expect(card.getByText(FAILURE_REASON, { exact: false })).toBeVisible({ timeout: 15_000 })
    await expect(card.getByText(/Yarrtube tries again at .+, in (9|10) min\./)).toBeVisible()
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
      expect.objectContaining({
        status: 'RETRYING',
        failed_attempts: 1,
        failure: expect.stringContaining(FAILURE_REASON),
      }),
    )
  })

  await test.step('Stop retrying on its Tasks view card makes the download Failed', async () => {
    await page.goto('/tasks')
    const card = page
      .getByRole('region', { name: 'Needs attention' })
      .locator('li', { hasText: new RegExp(`Download .+ from ${playlist.name}`) })
    const stopped = page.waitForResponse(
      (response) => response.request().method() === 'POST' && response.url().endsWith('/stop-retrying'),
    )
    await card.getByRole('button', { name: 'Stop retrying' }).click({ timeout: 15_000 })
    expect((await stopped).status()).toBe(204)
    await expect(card).toHaveCount(0)
    expect((await downloadStep()).status).toBe('FAILED')
    await page.goto(`/playlists/${encodeURIComponent(playlist.id)}`)
    await waitForVideoStatus(page, { status: 'FAILED' })
  })
})
