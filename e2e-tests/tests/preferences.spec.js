import { test, expect } from '../helpers/fixtures.js'
import { openAddChannelDialog } from '../helpers/addDialog.js'
import { sectionHeading } from '../helpers/sidebar.js'
import { addChannel, addPlaylist } from '../helpers/seed.js'
import { CHANNEL, PLAYLIST } from '../helpers/fakeYoutube.js'

/** The home section headed `title`. */
function homeSection(page, title) {
  return page.locator('main section', {
    has: page.getByRole('heading', { name: title, exact: true }),
  })
}

/** Flips one preference from the preferences view, then goes home through the logo. */
async function toggleThenGoHome(page, name) {
  await page.goto('/preferences')
  const toggle = page.getByRole('switch', { name })
  const before = await toggle.getAttribute('aria-checked')
  await toggle.click()
  await expect(toggle).not.toHaveAttribute('aria-checked', before)
  await page.getByRole('link', { name: 'Yarrtube', exact: true }).click()
}

test('preferences: turning categories off hides them everywhere, and back on shows them again', async ({ page }) => {
  await page.goto('/')
  const header = page.getByRole('banner')
  const enableCategories = page.getByRole('switch', { name: 'Enable categories' })

  await test.step('the settings menu opens the preferences view', async () => {
    await header.getByRole('button', { name: 'Settings' }).click()
    await expect(page.getByRole('menuitem', { name: 'Report a bug' })).toHaveAttribute(
      'href',
      'https://github.com/sergigp/yarrtube/issues/new',
    )
    await page.getByRole('menuitem', { name: 'Preferences', exact: true }).click()
    await expect(page).toHaveURL(/\/preferences$/)
    await expect(enableCategories).toBeChecked()
  })

  await test.step('categories off: no sidebar section, no dialog field, and it sticks', async () => {
    await enableCategories.click()
    await expect(enableCategories).not.toBeChecked()
    await expect(page.getByRole('switch', { name: 'Show categories' })).toBeDisabled()
    await expect(sectionHeading(page, 'Categories')).toHaveCount(0)

    await openAddChannelDialog(page)
    const dialog = page.getByRole('dialog')
    await expect(dialog.getByRole('group', { name: 'Categories' })).toHaveCount(0)
    await page.keyboard.press('Escape')
    await dialog.waitFor({ state: 'hidden' })

    await page.reload()
    await expect(enableCategories).not.toBeChecked()
    await expect(sectionHeading(page, 'Categories')).toHaveCount(0)
  })

  await test.step('categories back on: the sidebar section returns', async () => {
    await enableCategories.click()
    await expect(enableCategories).toBeChecked()
    await expect(sectionHeading(page, 'Categories')).toBeVisible()
  })
})

test('preferences: each home switch takes its section off home, and its videos move down', async ({ page, request }) => {
  // The playlist's only video is started, so it is continued; the channel's
  // two are in "Snails", so they get a category section. Every fixture clip
  // lasts 61 s, so each unwatched video is also a quick watch.
  const playlist = await addPlaylist(request, PLAYLIST)
  const started = playlist.videos[0]
  const progress = await request.post(`/api/videos/${started.id}/progress`, {
    data: { position_seconds: 40, duration_seconds: 61, was_watched: false },
  })
  expect(progress.ok(), await progress.text()).toBeTruthy()
  const channel = await addChannel(request, CHANNEL, { videoLimit: 2 })
  const category = await request.post('/api/categories', { data: { name: 'Snails' } })
  expect(category.ok(), await category.text()).toBeTruthy()
  const members = await request.put(`/api/categories/${(await category.json()).id}/members`, {
    data: { channels: [CHANNEL.handle], playlists: [] },
  })
  expect(members.ok(), await members.text()).toBeTruthy()
  const channelTitles = channel.videos.map((video) => video.title)
  const continueWatching = homeSection(page, 'Continue watching')
  const moreAboutSnails = homeSection(page, 'More about Snails')
  const quickWatches = homeSection(page, 'Quick watches')
  const latest = homeSection(page, 'Latest videos')
  const main = page.locator('main')

  /** Each of `titles` shows once on home: a moved video left its old section. */
  async function expectOnceOnHome(titles) {
    for (const title of titles) {
      await expect(main.getByText(title, { exact: true })).toHaveCount(1)
    }
  }
  const everyTitle = [started.title, ...channelTitles]

  await test.step('everything on: continued, then the category section', async () => {
    await page.goto('/')
    await expect(continueWatching.getByText(started.title, { exact: true })).toBeVisible()
    for (const title of channelTitles) {
      await expect(moreAboutSnails.getByText(title, { exact: true })).toBeVisible()
    }
    await expectOnceOnHome(everyTitle)
  })

  await test.step('categories off: no category section, its videos are quick watches', async () => {
    await toggleThenGoHome(page, 'Enable categories')
    for (const title of channelTitles) {
      await expect(quickWatches.getByText(title, { exact: true })).toBeVisible()
    }
    await expect(main.getByRole('heading', { name: /^More about/ })).toHaveCount(0)
    await expectOnceOnHome(everyTitle)
  })

  await test.step('categories back on: the category section returns', async () => {
    await toggleThenGoHome(page, 'Enable categories')
    for (const title of channelTitles) {
      await expect(moreAboutSnails.getByText(title, { exact: true })).toBeVisible()
    }
    await expectOnceOnHome(everyTitle)
  })

  await test.step('home categories off: the category section goes again', async () => {
    await toggleThenGoHome(page, 'Show categories')
    for (const title of channelTitles) {
      await expect(quickWatches.getByText(title, { exact: true })).toBeVisible()
    }
    await expect(main.getByRole('heading', { name: /^More about/ })).toHaveCount(0)
    await expectOnceOnHome(everyTitle)
  })

  await test.step('continue watching off: the started video is a quick watch', async () => {
    await toggleThenGoHome(page, 'Show continue watching')
    await expect(quickWatches.getByText(started.title, { exact: true })).toBeVisible()
    await expect(main.getByRole('heading', { name: 'Continue watching' })).toHaveCount(0)
    await expectOnceOnHome(everyTitle)
  })

  await test.step('quick watches off: every video is under latest videos', async () => {
    await toggleThenGoHome(page, 'Show quick watches')
    for (const title of everyTitle) {
      await expect(latest.getByText(title, { exact: true })).toBeVisible()
    }
    await expect(main.getByRole('heading', { name: 'Quick watches' })).toHaveCount(0)
    await expectOnceOnHome(everyTitle)
  })

  await test.step('latest videos off: nothing left to show', async () => {
    await toggleThenGoHome(page, 'Show latest videos')
    await expect(main.getByText('Nothing to show here.')).toBeVisible()
    await expect(main.getByRole('link', { name: 'Preferences' })).toHaveAttribute('href', '/preferences')
    await expect(main.getByRole('heading', { name: 'Latest videos' })).toHaveCount(0)
    for (const title of everyTitle) {
      await expect(main.getByText(title, { exact: true })).toHaveCount(0)
    }
  })

  await test.step('latest videos back on: the videos are back', async () => {
    await toggleThenGoHome(page, 'Show latest videos')
    for (const title of everyTitle) {
      await expect(latest.getByText(title, { exact: true })).toBeVisible()
    }
    await expect(main.getByText('Nothing to show here.')).toHaveCount(0)
    await expectOnceOnHome(everyTitle)
  })
})

test('preferences: the dark mode switch is saved on the daemon, not just in the browser', async ({ page }) => {
  await page.goto('/')
  const darkMode = page.getByRole('banner').getByRole('switch', { name: 'Dark mode' })
  await expect(darkMode).toBeEnabled()
  await expect(page.locator('html')).not.toHaveClass(/\bdark\b/)

  await darkMode.click()
  await expect(darkMode).toBeChecked()
  await expect(page.locator('html')).toHaveClass(/\bdark\b/)

  // With the browser's cache gone, only the saved preference can bring it back.
  await page.evaluate(() => window.localStorage.clear())
  await page.reload()
  await expect(page.locator('html')).toHaveClass(/\bdark\b/)
  await expect(darkMode).toBeChecked()

  await darkMode.click()
  await expect(page.locator('html')).not.toHaveClass(/\bdark\b/)
  await page.evaluate(() => window.localStorage.clear())
  await page.reload()
  await expect(darkMode).toBeEnabled()
  await expect(page.locator('html')).not.toHaveClass(/\bdark\b/)
})
