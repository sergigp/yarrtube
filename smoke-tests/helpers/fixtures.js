import { test as base, expect } from '@playwright/test'
import { recordApiRoute } from './routeCoverage.js'
import { libraryPath } from './library.js'
import { resetPlexStub } from './plex.js'
import { existsSync } from 'node:fs'

/**
 * Every spec imports `test`/`expect` from here instead of
 * `@playwright/test`, so each test:
 *
 * - starts from an empty library (unless it opts out with
 *   `test.use({ keepLibrary: true })`): whatever an earlier (possibly failed)
 *   test left tracked, and every category, is deleted through the API, and its files are waited
 *   out, so one failure can't cascade into "Already added" or ambiguous
 *   sidebar rows in the tests after it (or in its own retry), and the stub
 *   Plex server forgets the scans earlier tests caused;
 * - fails on an uncaught page error, a `console.error` not in
 *   `consoleErrorAllowlist`, or any 5xx `/api` response, which no individual
 *   assertion would otherwise notice;
 * - records the `/api` routes the browser hit, checked against the router
 *   by `routeCoverageReporter.js`.
 */
export const test = base.extend({
  keepLibrary: [false, { option: true }],
  // `/api` 4xx responses the test triggers on purpose, e.g.
  // `[{ method: 'POST', path: '/api/playlists', status: 409 }]`.
  expectedApiErrors: [[], { option: true }],
  // `console.error` messages the test tolerates, as RegExps.
  consoleErrorAllowlist: [[], { option: true }],

  cleanLibrary: [
    async ({ request, keepLibrary }, use) => {
      if (!keepLibrary) {
        await deleteAllSources(request)
        await resetPlexStub(request)
      }
      await use()
    },
    { auto: true },
  ],

  page: async ({ page, expectedApiErrors, consoleErrorAllowlist }, use) => {
    const problems = []
    page.on('pageerror', (error) => problems.push(`page error: ${error.message}`))
    page.on('console', (message) => {
      // A failed resource load is logged as an error too; the request checks
      // below own those, and know which ones the test expects.
      if (message.type() !== 'error' || message.text().startsWith('Failed to load resource')) {
        return
      }
      // Playwright's WebKit logs these for its own native video controls'
      // icons (picture in picture, AirPlay), whatever the page does.
      if (/^Button failed to load, iconName = [\w-]+-placard,/.test(message.text())) {
        return
      }
      if (!consoleErrorAllowlist.some((allowed) => allowed.test(message.text()))) {
        problems.push(`console.error: ${message.text()}`)
      }
    })
    page.on('response', (response) => {
      const url = new URL(response.url())
      if (!url.pathname.startsWith('/api/')) {
        return
      }
      recordApiRoute(response.request().method(), url.pathname)
      if (response.status() >= 500) {
        problems.push(`${response.status()} from ${response.request().method()} ${url.pathname}`)
      }
    })
    await use(page)
    expect(problems, 'page errors, console errors and failed requests during the test').toEqual([])
  },
})

export { expect }

export async function deleteAllSources(request) {
  await deleteAllCategories(request)
  const leftovers = []
  for (const kind of ['playlists', 'channels']) {
    const response = await request.get(`/api/${kind}`)
    expect(response.ok(), `GET /api/${kind}`).toBeTruthy()
    for (const source of await response.json()) {
      const deleted = await request.delete(`/api/${kind}/${encodeURIComponent(source.id)}`)
      expect(deleted.ok(), `DELETE /api/${kind}/${source.id}`).toBeTruthy()
      leftovers.push(libraryPath(source.path))
    }
  }
  // Deleting schedules the folder's removal; wait for it, so a test that
  // re-adds the same source doesn't race the removal of its old folder.
  await expect
    .poll(() => leftovers.filter((dir) => existsSync(dir)), { timeout: 30_000 })
    .toEqual([])
}

// The upgrade check's seed phase runs against a released image that may
// predate categories, where the path falls through to the web UI's HTML.
async function deleteAllCategories(request) {
  const response = await request.get('/api/categories')
  if (!(response.headers()['content-type'] ?? '').includes('application/json')) {
    return
  }
  expect(response.ok(), 'GET /api/categories').toBeTruthy()
  for (const category of (await response.json()).categories) {
    const deleted = await request.delete(`/api/categories/${encodeURIComponent(category.id)}`)
    expect(deleted.ok(), `DELETE /api/categories/${category.id}`).toBeTruthy()
  }
}
