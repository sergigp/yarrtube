import { test, expect } from '../helpers/fixtures.js'

test('shows the running version in the header', async ({ page }) => {
  await page.goto('/')

  // The smoke image is built without YARRTUBE_VERSION, so it reports `dev`,
  // and the header shows exactly what the API reports.
  const { version } = await (await page.request.get('/api/version')).json()
  expect(version).toBe('dev')
  await expect(page.getByRole('banner').getByText(version, { exact: true })).toBeVisible()
})
