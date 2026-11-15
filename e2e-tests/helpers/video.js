import { expect } from '@playwright/test'

// The detail pane shows a status badge only for videos that are not
// downloaded; a downloaded one is recognised by its speed control, which is
// disabled until then.
const STATUS_LABELS = {
  IN_PROGRESS: 'Downloading',
  PENDING: 'Pending',
  RETRYING: 'Retrying',
  FAILED: 'Failed',
  UNAVAILABLE: 'Unavailable',
}

/**
 * On a playlist's or category's browse page, opens its first card into the
 * watching layout (player and queue), waiting up to `timeoutMs` for a video
 * to be listed; a page already watching (a channel's) is left as it is.
 */
export async function enterWatching(page, { timeoutMs = 30_000 } = {}) {
  const main = page.locator('main')
  const queue = main.getByRole('complementary', { name: 'Queue' })
  const firstCard = main.locator('li:has([data-slot="card-thumbnail"]) a').first()
  await expect(queue.or(firstCard).first()).toBeVisible({ timeout: timeoutMs })
  if (!(await queue.isVisible())) {
    await firstCard.click()
    await expect(queue).toBeVisible()
  }
}

/**
 * Leaves the watching layout of the playlist named `name` for its browse page,
 * through the name in its header, so no video plays: a video still playing
 * when its playlist is deleted reports its progress to a video that is gone.
 */
export async function leaveWatching(page, name) {
  await page.locator('main').getByRole('link', { name, exact: true }).click()
  await expect(page.locator('video')).toHaveCount(0)
}

export async function waitForVideoStatus(page, { title, status = 'DOWNLOADED', timeoutMs = 30_000 } = {}) {
  const main = page.locator('main')
  await enterWatching(page, { timeoutMs })
  if (title) {
    await main.getByRole('button').filter({ hasText: title }).first().click()
    // The detail pane shows the video asked about, so its status is the one read.
    await expect(main.getByRole('heading', { level: 3 }).first()).toHaveText(title)
  }
  const indicator =
    status === 'DOWNLOADED'
      ? main.getByRole('button', { name: 'Playback speed', disabled: false })
      : main.getByText(STATUS_LABELS[status] ?? status, { exact: true }).first()
  if (status !== 'DOWNLOADED') {
    await expect(indicator).toBeVisible({ timeout: timeoutMs })
    return
  }
  // A failed download waits at least 10 minutes for its next attempt, and one
  // that gave up won't recover, so fail on either right away rather than
  // after the whole timeout.
  const failing = main
    .getByText(STATUS_LABELS.RETRYING, { exact: true })
    .or(main.getByText(STATUS_LABELS.FAILED, { exact: true }))
    .first()
  await expect(indicator.or(failing).first()).toBeVisible({ timeout: timeoutMs })
  await expect(failing, 'the download failed').toBeHidden()
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

/**
 * Picks `name` from the page header's "⋮" menu, where every action but
 * Sync, the YouTube link and a category's "Edit category" lives.
 */
export async function chooseHeaderAction(page, name) {
  await page.locator('main').getByRole('button', { name: /^More actions for / }).click()
  await page.getByRole('menuitem', { name, exact: true }).click()
}
