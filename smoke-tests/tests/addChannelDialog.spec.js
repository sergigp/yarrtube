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
