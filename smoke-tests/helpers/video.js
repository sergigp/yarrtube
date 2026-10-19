import { expect } from '@playwright/test'

// The detail pane shows a status badge only for videos that are not
// downloaded; a downloaded one is recognised by the meta line's "Synced" part.
const STATUS_LABELS = {
  IN_PROGRESS: 'Downloading',
  PENDING: 'Pending',
  ERRORED_RETRYING: 'Retrying',
  ERRORED: 'Errored',
}

export async function waitForVideoStatus(page, { title, status = 'DOWNLOADED', timeoutMs = 240_000 } = {}) {
  const main = page.locator('main')
  if (title) {
    await main.getByRole('button').filter({ hasText: title }).first().click()
    // The detail pane shows the video asked about, so its status is the one read.
    await expect(main.getByRole('heading', { level: 3 }).first()).toHaveText(title)
  }
  const indicator =
    status === 'DOWNLOADED'
      ? main.getByText(/^Synced /).first()
      : main.getByText(STATUS_LABELS[status] ?? status, { exact: true }).first()
  if (status !== 'DOWNLOADED') {
    await expect(indicator).toBeVisible({ timeout: timeoutMs })
    return
  }
  // A download that has given up won't recover, so fail on it right away
  // rather than after the whole timeout.
  const errored = main.getByText(STATUS_LABELS.ERRORED, { exact: true }).first()
  await expect(indicator.or(errored).first()).toBeVisible({ timeout: timeoutMs })
  await expect(errored, 'the download errored').toBeHidden()
}

export async function assertVideoPlays(page) {
  const video = page.locator('video')
  await expect(video).toBeVisible()
  await video.evaluate(
    (el) =>
      new Promise((resolve, reject) => {
        if (el.readyState >= 1) {
          resolve()
          return
        }
        el.addEventListener('loadedmetadata', () => resolve(), { once: true })
        el.addEventListener('error', () => reject(new Error('video element failed to load')), {
          once: true,
        })
      }),
  )
  await expect.poll(() => video.evaluate((el) => el.duration)).toBeGreaterThan(0)

  await video.evaluate((el) => el.play())
  await expect.poll(() => video.evaluate((el) => el.currentTime), { timeout: 15_000 }).toBeGreaterThan(0)
}

/**
 * Waits for the player to offer the selected video's subtitles (they're
 * fetched after the download, and the video list polls), turns them on, and
 * checks the browser loads them as WebVTT cues.
 */
export async function assertSubtitlesShow(page, { timeoutMs = 120_000 } = {}) {
  const video = page.locator('video')
  await expect(video.locator('track[kind="subtitles"]')).toBeAttached({ timeout: timeoutMs })

  const subtitlesLoaded = page.waitForResponse((response) => response.url().endsWith('/subtitles.vtt'))
  // Captions start off: turning them on is what makes the browser fetch them.
  await video.evaluate((el) => {
    el.textTracks[0].mode = 'showing'
  })
  const response = await subtitlesLoaded
  expect(response.status()).toBe(200)
  expect(response.headers()['content-type']).toBe('text/vtt; charset=utf-8')
  expect(await response.text()).toMatch(/^WEBVTT\n/)
  await expect.poll(() => video.evaluate((el) => el.textTracks[0].cues?.length ?? 0)).toBeGreaterThan(0)
}
