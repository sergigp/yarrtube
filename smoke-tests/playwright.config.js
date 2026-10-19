import { defineConfig, devices } from '@playwright/test'

// Without these the lifecycle specs can't run, and silently skipping them
// would leave a green suite that never downloaded anything.
const REQUIRED_ENV = ['SMOKE_PLAYLIST_ID', 'SMOKE_VIDEOS_DIR', 'SMOKE_PLEX_STUB_URL']
const missing = REQUIRED_ENV.filter((name) => !process.env[name])
if (missing.length > 0) {
  throw new Error(`smoke tests need ${missing.join(', ')} set (see smoke-tests/README.md)`)
}

// `run-smoke-tests.sh` runs the upgrade check's phases (`upgrade/seed.spec.js`
// against the released image, `upgrade/verify.spec.js` against this build)
// on their own, before the main suite.
const UPGRADE_PHASE = process.env.SMOKE_UPGRADE_PHASE

export default defineConfig({
  ...(UPGRADE_PHASE
    ? { testDir: './upgrade', testMatch: `${UPGRADE_PHASE}.spec.js` }
    : { testDir: './tests' }),
  fullyParallel: false,
  workers: 1,
  retries: process.env.CI ? 2 : 0,
  // Lifecycle tests wait up to 240s for a real yt-dlp download to finish
  // (enough to cover a couple of retries at the smoke suite's short retry
  // base delay), on top of the dialog/sync/delete steps around it.
  timeout: 300_000,
  reporter: [['list'], ['./routeCoverageReporter.js']],
  use: {
    baseURL: process.env.BASE_URL ?? 'http://localhost:8080',
    trace: 'retain-on-failure',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
})
