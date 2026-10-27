## 1. Walking skeleton

- [x] 1.1 Create `e2e-tests/helpers/daemon.js` (`startDaemon` returning the current single container's URL and env, `stop` doing nothing), export `startPlexStub` from `plex-stub.js` (standalone run unchanged), the worker-scoped `daemon` fixture and `baseURL` from it in `fixtures.js`, call-time env reads in `library.js`/`plex.js`, `E2E_WORKERS` (default 1) in `playwright.config.js`, and `--workers` parsed and passed by `scripts/run-tests.sh` (the script still starts the main-suite container). Done when `scripts/run-tests.sh --only e2e --workers 1` passes as today.

## 2. Behaviour (TDD)

- [x] 2.1 Each worker starts its own daemon: `startDaemon` runs the container, Plex stub and temp dirs, `stop` saves the log, checks for a panic and removes everything; the script stops starting the main-suite container. `scripts/run-tests.sh --only e2e --workers 1` passes (65).
- [x] 2.2 `fullyParallel: true`; `scripts/run-tests.sh --only e2e --workers 3` passes, with at most 3 of the run's daemons alive at once and none left afterwards.
- [ ] 2.3 Defaults: half the CPU cores (at most 4) locally, 2 under `--ci`; invalid `--workers` values rejected. Time `--workers 1, 2, 3, 4` locally and run the default 3 times without a flaky test; fix or slow down any timing-sensitive test that flakes under load.
- [ ] 2.4 `ci.yml` passes `--workers <measured>`; the GitHub run passes and its e2e step time is recorded (needs a push).

## 3. Infrastructure adapters (TDD)

None: no Rust changes.

## 4. Verification

- [ ] 4.1 `e2e-tests/README.md` (workers, containers per run, per-worker logs) and one line in `doc/DEVELOPMENT.md` for `--workers`; `scripts/run-tests.sh` passes locally (upgrade check included).
