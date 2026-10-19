import { expect } from '@playwright/test'

// The stub Plex server `run-smoke-tests.sh` starts (see `plex-stub.js`), and
// what the container is configured to reach it with.
const PLEX_STUB_URL = process.env.SMOKE_PLEX_STUB_URL
const PLEX_TOKEN = 'smoke-token'
const PLEX_VIDEOS_PATH = '/plex/yarrtube'
const SECTION_FOR = { playlists: '1', channels: '2' }

async function plexRequests(request) {
  const response = await request.get(`${PLEX_STUB_URL}/__requests`)
  expect(response.ok()).toBeTruthy()
  return response.json()
}

/** Forgets what the stub recorded, so a test only sees its own scans. */
export async function resetPlexStub(request) {
  const response = await request.post(`${PLEX_STUB_URL}/__reset`)
  expect(response.ok()).toBeTruthy()
}

/**
 * Asserts Plex was asked to scan the show at `showPath` (relative to the
 * videos root, e.g. `playlists/test`): at its Plex-side path, in the one
 * section holding it, with the configured token. Scans arrive after the
 * download event is handled, so this waits for one.
 */
export async function expectPlexScanned(request, showPath) {
  const plexPath = `${PLEX_VIDEOS_PATH}/${showPath}`
  const section = SECTION_FOR[showPath.split('/')[0]]
  const scansOf = (requests) =>
    requests.filter((r) => r.path.endsWith('/refresh') && r.query.path === plexPath)

  await expect
    .poll(async () => scansOf(await plexRequests(request)).length, {
      message: `Plex scan of ${plexPath}`,
      timeout: 30_000,
    })
    .toBeGreaterThan(0)

  const requests = await plexRequests(request)
  // Downloads and subtitle fetches each trigger a scan, so there can be
  // several; every one must target the right section with the token.
  for (const scan of scansOf(requests)) {
    expect(scan).toEqual({
      method: 'GET',
      path: `/library/sections/${section}/refresh`,
      query: { path: plexPath },
      token: PLEX_TOKEN,
    })
  }
  for (const listing of requests.filter((r) => r.path === '/library/sections')) {
    expect(listing.token).toBe(PLEX_TOKEN)
  }
}
