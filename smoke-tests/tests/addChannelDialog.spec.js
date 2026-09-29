import { test, expect } from '@playwright/test'
import { openAddChannelDialog, destinationNotice } from '../helpers/addDialog.js'

test('it should show no notice until a handle is entered', async ({ page }) => {
  await page.goto('/')
  await openAddChannelDialog(page)
  const dialog = page.getByRole('dialog')

  await expect(destinationNotice(dialog)).toHaveCount(0)

  await dialog.getByLabel('Channel Handle or URL').fill('@some-handle')
  await expect(destinationNotice(dialog)).toBeVisible()
})

test('it should state the video limit and destination for a handle', async ({ page }) => {
  await page.goto('/')
  await openAddChannelDialog(page)
  const dialog = page.getByRole('dialog')

  await dialog.getByLabel('Channel Handle or URL').fill('@some-handle')

  const notice = destinationNotice(dialog)
  await expect(notice).toContainText('The latest 3 videos')
  // Absolute: the videos root, the default `channels` parent and the slug.
  await expect(notice.locator('code')).toHaveText(/^\/.+\/channels\/some-handle$/)
  await expect(dialog.getByRole('button', { name: /Advanced options/ })).toHaveAttribute(
    'aria-expanded',
    'false',
  )
})

test('it should update the notice from the advanced options', async ({ page }) => {
  await page.goto('/')
  await openAddChannelDialog(page)
  const dialog = page.getByRole('dialog')
  await dialog.getByLabel('Channel Handle or URL').fill('@some-handle')
  await dialog.getByRole('button', { name: /Advanced options/ }).click()

  await dialog.getByLabel('Video Limit').fill('1')
  await dialog.getByLabel('Folder name').fill('renamed')

  const notice = destinationNotice(dialog)
  await expect(notice).toContainText('The latest video from this channel will be downloaded to')
  await expect(notice.locator('code')).toHaveText(/^\/.+\/channels\/renamed$/)

  // Out of range, the count is left out rather than stated wrongly.
  await dialog.getByLabel('Video Limit').fill('0')
  await expect(notice).toContainText('Videos from this channel will be downloaded to')
})
