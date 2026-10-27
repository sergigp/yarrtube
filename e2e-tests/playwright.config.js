import { defineConfig, devices } from '@playwright/test'

// `scripts/run-tests.sh` runs the upgrade check's phases (`upgrade/seed.spec.js`
// against the released image, `upgrade/verify.spec.js` against this build)
// on their own, before the main suite.
const UPGRADE_PHASE = process.env.E2E_UPGRADE_PHASE

// Without these the specs can't run, and silently skipping them would leave a
// green suite that never downloaded anything. Only the upgrade check uses a
// playlist on the real YouTube.
const REQUIRED_ENV = [
  'E2E_VIDEOS_DIR',
  'E2E_PLEX_STUB_URL',
  ...(UPGRADE_PHASE ? ['E2E_PLAYLIST_ID'] : ['E2E_YOUTUBE_STUB_PORT']),
]
const missing = REQUIRED_ENV.filter((name) => !process.env[name])
if (missing.length > 0) {
  throw new Error(`end-to-end tests need ${missing.join(', ')} set (see e2e-tests/README.md)`)
}

export default defineConfig({
  ...(UPGRADE_PHASE
    ? { testDir: './upgrade', testMatch: `${UPGRADE_PHASE}.spec.js` }
    : { testDir: './tests' }),
  fullyParallel: false,
  // `scripts/run-tests.sh --workers N`; one when run directly.
  workers: Number(process.env.E2E_WORKERS ?? 1),
  retries: process.env.CI ? 2 : 0,
  // Against the fake YouTube a download takes about a second; the longest
  // test waits up to a minute for the sidebar's periodic refresh. The
  // upgrade check's seeding still downloads from the real YouTube, for up to
  // 240 s.
  timeout: UPGRADE_PHASE ? 300_000 : 120_000,
  reporter: [['list'], ['./routeCoverageReporter.js']],
  use: {
    baseURL: process.env.BASE_URL ?? 'http://localhost:8080',
    trace: 'retain-on-failure',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  // Specs tagged `@mobile` run only on an emulated iPhone (WebKit, touch,
  // mobile viewport); everything else only on desktop Chromium, whose
  // layout has no drawer.
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
      grepInvert: /@mobile/,
    },
    {
      name: 'mobile-safari',
      use: { ...devices['iPhone 13'] },
      grep: /@mobile/,
    },
  ],
})
