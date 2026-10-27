## Why

The end-to-end suite is now the longest part of CI (~5.4 min of a ~9 min run on GitHub, ~4.5 min locally) and runs one test at a time against a single daemon, although it is mostly waiting (sidebar refreshes, progress reports, retries): two full runs at once on a laptop took 4.6 min each against 4.5 min alone. Running it on several Playwright workers, each with its own daemon, should roughly halve it in CI at no extra machine cost, and more locally.

## What Changes

- Each Playwright worker starts its own yarrtube container (fresh database and videos directory, free ports, its own Plex stub) and removes it when it ends; tests in a worker talk only to that worker's daemon. The read-only fake YouTube Data API stays one shared process. At most N containers are alive at once; a run starts about one per worker per browser project, plus one per failed test (Playwright replaces a worker after a failure).
- Tests run fully in parallel across workers (`fullyParallel`), since each test already starts from an empty library.
- `scripts/run-tests.sh --workers <N>` sets the number of end-to-end workers: 3 by default, locally and in CI (measured: on 14 cores 1 → 4.6 min, 3 → 1.6 min; on GitHub's 4 vCPUs 3 workers → 2.0 min). The script also ends with a summary: each phase's time and test counts. `--workers 1` runs as today.
- The script no longer starts a container for the main suite; the upgrade check (only with a YouTube API key) still runs before the suite, in its own containers, one after the other.
- Container logs are saved per worker in `e2e-tests/test-results/`; a panic in any worker's daemon fails the run.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `e2e-tests`: "Isolated Runs" extended to a daemon per worker; a new requirement for running the suite on a configurable number of parallel workers.

## Impact

- `e2e-tests/helpers/fixtures.js` (worker-scoped daemon), `e2e-tests/plex-stub.js` (startable from a worker), `e2e-tests/helpers/library.js` and `helpers/plex.js` (read the worker's values at call time), `e2e-tests/playwright.config.js` (workers, `fullyParallel`), `scripts/run-tests.sh` (`--workers`, no main-suite container), `.github/workflows/ci.yml`, `e2e-tests/README.md`, `doc/DEVELOPMENT.md`.
- No production code changes.
- Risk: CPU contention on GitHub's 4 vCPUs could slow timing-sensitive tests; the fallback is fewer workers.
