import { test, expect } from '@playwright/test'
import { openAddChannelDialog, destinationNotice, slugOf } from '../helpers/addDialog.js'

const CHANNEL_HANDLE = process.env.SMOKE_CHANNEL_HANDLE ?? '@BlenderOfficial'

test('it should show no notice until a handle is entered', async ({ page }) => {
  await page.goto('/')
  await openAddChannelDialog(page)
  const dialog = page.getByRole('dialog')

  await expect(destinationNotice(dialog)).toHaveCount(0)

  await dialog.getByLabel('Channel Handle or URL').fill(CHANNEL_HANDLE)
  await expect(destinationNotice(dialog)).toBeVisible()
})

test('it should explain a value that is not a channel', async ({ page }) => {
  await page.goto('/')
  await openAddChannelDialog(page)
  const dialog = page.getByRole('dialog')

  await dialog.getByLabel('Channel Handle or URL').fill('somechannel')

  const notice = destinationNotice(dialog)
  await expect(notice).toHaveText('Channel handle must start with "@" (got "somechannel")')
  await expect(notice).toHaveClass(/text-destructive/)
  await expect(dialog.getByRole('button', { name: /^Create Channel/ })).toBeDisabled()
})

test('it should state the video limit and destination for a handle', async ({ page }) => {
  await page.goto('/')
  await openAddChannelDialog(page)
  const dialog = page.getByRole('dialog')

  await dialog.getByLabel('Channel Handle or URL').fill(CHANNEL_HANDLE)

  const notice = destinationNotice(dialog)
  await expect(notice).toContainText('The latest 3 videos')
  // Absolute: the videos root, the default `channels` parent and the slug.
  await expect(notice.locator('code')).toHaveText(new RegExp(`^/.+/channels/${slugOf(CHANNEL_HANDLE)}$`))
  await expect(dialog.getByRole('button', { name: /Advanced options/ })).toHaveAttribute(
    'aria-expanded',
    'false',
  )
})

test('it should update the notice from the advanced options', async ({ page }) => {
  await page.goto('/')
  await openAddChannelDialog(page)
  const dialog = page.getByRole('dialog')
  await dialog.getByLabel('Channel Handle or URL').fill(CHANNEL_HANDLE)
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

test('it should expand advanced options from the change action', async ({ page }) => {
  await page.goto('/')
  await openAddChannelDialog(page)
  const dialog = page.getByRole('dialog')
  await dialog.getByLabel('Channel Handle or URL').fill(CHANNEL_HANDLE)

  await destinationNotice(dialog).getByRole('button', { name: 'change' }).click()

  await expect(dialog.getByRole('button', { name: /Advanced options/ })).toHaveAttribute(
    'aria-expanded',
    'true',
  )
  await expect(dialog.getByLabel('Folder name')).toBeVisible()
  await expect(dialog.getByLabel('Folder name')).toHaveValue(slugOf(CHANNEL_HANDLE))
})
