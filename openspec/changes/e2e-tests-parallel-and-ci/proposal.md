## Why

Even with a fake YouTube, the end-to-end tests run one at a time, because every test shares one daemon and one library (each test deletes every source, and many assertions are about global state). They also don't run in CI: the workflow is manual-only because real YouTube bot-checks GitHub's runners, although the `e2e-tests` spec's "Continuous Integration" requirement says it runs on every PR. Once YouTube is faked, both can be fixed.

## What Changes

- One daemon per Playwright worker: a worker-scoped fixture starts its own container (own database, videos directory and port) and Plex stub (or one stub keyed by a per-worker token), and passes `baseURL`, the videos directory and the container name to its tests. Tests inside a worker keep today's assumptions; workers share nothing, so the suite runs with several workers.
- An e2e-tests job in CI on every pull request and push to `main`, without `YOUTUBE_API_KEY`, building the image with BuildKit layer caching and installing Chromium and WebKit; a required check before `release.yml` publishes `:edge`.
- The manual real-YouTube workflow is removed, or kept only for the canary and contract tests before releases.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `e2e-tests`: parallel workers with isolated daemons; the "Continuous Integration" requirement made true (PRs and `main`, no API key).
- `release-channels`: publishing `:edge` waits for the e2e-tests check.

## Impact

- `e2e-tests/playwright.config.js`, `helpers/fixtures.js` (worker-scoped daemon), the run script, `.github/workflows/ci.yml` / `e2e-tests.yml`, `release.yml`.
- More memory and CPU per run locally (N containers); the worker count may need a sensible default.
- Depends on `acceptance-tests-fake-youtube`.
