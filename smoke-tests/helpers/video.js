import { expect } from '@playwright/test'

const STATUS_LABELS = {
  DOWNLOADED: 'Downloaded',
  IN_PROGRESS: 'Downloading',
  PENDING: 'Pending',
  ERRORED_RETRYING: 'Retrying',
  ERRORED: 'Errored',
}

export async function waitForVideoStatus(page, { title, status = 'DOWNLOADED', timeoutMs = 240_000 } = {}) {
  if (title) {
    await page.getByRole('button').filter({ hasText: title }).first().click()
  }
  const label = STATUS_LABELS[status] ?? status
  await expect(page.getByText(label, { exact: true }).first()).toBeVisible({ timeout: timeoutMs })
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
