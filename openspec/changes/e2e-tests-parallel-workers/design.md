## Files

- `e2e-tests/helpers/daemon.js` (new): starts and stops one worker's daemon: temp videos/config dirs, free ports, an in-process Plex stub, `docker run` of `E2E_IMAGE` with the fake-YouTube settings (the ones `run-tests.sh` passes today), wait for `/status`; on stop, save the container's log to `test-results/container-<run id>-w<worker>.log`, fail on `panicked at`, remove the container and dirs.
- `e2e-tests/helpers/fixtures.js`: a worker-scoped `daemon` fixture (auto) built on `daemon.js`; `baseURL` comes from it. During the upgrade check's phases (`E2E_UPGRADE_PHASE` set) it starts nothing and uses the script's container (`BASE_URL` and the `E2E_*` the script sets), as today.
- `e2e-tests/plex-stub.js`: exports `startPlexStub(port)`; still runs standalone (the upgrade check's container uses one the script starts).
- `e2e-tests/helpers/library.js`, `e2e-tests/helpers/plex.js`: read `E2E_VIDEOS_DIR` / `E2E_PLEX_STUB_URL` when called instead of at import, so the `daemon` fixture can set them for its own worker process (each worker is a separate process).
- `e2e-tests/playwright.config.js`: `workers` from `E2E_WORKERS` (default 1), `fullyParallel: true`; the main suite no longer requires `E2E_VIDEOS_DIR` / `E2E_PLEX_STUB_URL` (each worker makes its own) but requires `E2E_IMAGE` and `E2E_YOUTUBE_STUB_PORT`.
- `scripts/run-tests.sh`: `--workers <N>` (positive integer; default 3, locally and in CI), passed as `E2E_WORKERS`; a closing summary of each phase's time and test counts (counted from each phase's output, also printed when a phase fails); the e2e suite builds (or takes) the image, starts the shared YouTube stub, runs the upgrade check in its own containers when there's a key, then runs Playwright with `E2E_IMAGE` and `E2E_RUN_ID`; the main-suite container, its Plex stub and the panic check move to the workers.
- `.github/workflows/ci.yml`: `scripts/run-tests.sh --ci --workers <measured>`.
- `e2e-tests/README.md`, `doc/DEVELOPMENT.md`: workers (one line in DEVELOPMENT).

## Types & Signatures

```js
// e2e-tests/helpers/daemon.js
export async function startDaemon({ runId, workerIndex, image, youtubeStubPort })
  // → { url, videosDir, plexStubUrl, stop: async () => void }

// e2e-tests/plex-stub.js
export function startPlexStub(port) // → Promise<{ port, close: () => Promise<void> }>

// e2e-tests/helpers/fixtures.js
daemon:  [async ({}, use, workerInfo) => …, { scope: 'worker', auto: true }]
baseURL: async ({ daemon }, use) => use(daemon.url)
```

```bash
scripts/run-tests.sh [--ci] [--workers N] [--only <suite>]...
```

## Call Stack

- `run-tests.sh --only e2e --workers 3` → build image `yarrtube:e2e-<run id>` (or `E2E_IMAGE`) → start shared YouTube stub (free port) → upgrade check if `YOUTUBE_API_KEY` (script containers, sequential) → `E2E_WORKERS=3 E2E_IMAGE=… E2E_RUN_ID=… E2E_YOUTUBE_STUB_PORT=… npm --prefix e2e-tests test`.
- Playwright worker k (per browser project) → `daemon` fixture → `startDaemon({ runId, workerIndex: k, image, youtubeStubPort })` → `startPlexStub(free port)` + `docker run --name yarrtube-e2e-<run id>-w<k>-<n> -p <free>:<free> … image` → wait `/status` → set `process.env.E2E_VIDEOS_DIR`, `E2E_PLEX_STUB_URL` → tests (`baseURL` = daemon url; `cleanLibrary` per test as today) → worker ends → `stop()`: save log, check panic, `docker rm -f`, remove dirs, close Plex stub.
- Route coverage: every worker appends to `test-results/api-routes-hit.txt` (one line per append); the reporter reads it once in `onEnd`, as today.

## Test Plan

No application changes, so no Rust tests; the suite runs are the checks:

1. Behaviour (the suite):
   1. `--workers 1` passes (65 tests), one container at a time, as today.
   2. `--workers 3` passes; while it runs, `docker ps` shows at most 3 of the run's daemons, and none remain afterwards.
   3. Timing: `--workers 1, 2, 3, 4` locally, recorded; the suite run 3 times at the default count without a flaky test.
   4. A worker's container log is kept in `test-results/`, and a panic fails the run (checked by grepping the saved logs of a passing run for the panic check's line, and by a throwaway forced failure).
   5. `--ci` on GitHub with the measured worker count passes; its e2e step time recorded.
