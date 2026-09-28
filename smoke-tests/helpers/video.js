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
  if (title) {
    await page.getByRole('button').filter({ hasText: title }).first().click()
  }
  const indicator =
    status === 'DOWNLOADED'
      ? page.getByText(/^Synced /).first()
      : page.getByText(STATUS_LABELS[status] ?? status, { exact: true }).first()
  await expect(indicator).toBeVisible({ timeout: timeoutMs })
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
