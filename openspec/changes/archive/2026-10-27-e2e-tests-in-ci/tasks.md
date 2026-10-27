## 1. Walking skeleton

- [x] 1.1 Fold `run-youtube-contract-tests.sh` and `run-e2e-tests.sh` into `scripts/run-tests.sh` as the `contract` and `e2e` suites (`--only`, `--ci` skipping only `contract`, `E2E_IMAGE`, prerequisites checked instead of installed), remove the two scripts and `.github/workflows/e2e-tests.yml`, and update every reference. Done when `git grep "run-e2e-tests\|run-youtube-contract-tests"` finds nothing outside archived changes and `scripts/run-tests.sh --only web` passes.

## 2. Behaviour (TDD)

- [x] 2.1 `scripts/run-tests.sh --only contract` (16 passed) and `scripts/run-tests.sh --only e2e` (65 passed) each run just that suite. Recorded: `--only contract` 16 passed in 17 s; `--only e2e` 65 passed in 4m47s.
- [x] 2.2 `CI=true E2E_IMAGE=<prebuilt image> scripts/run-tests.sh` runs web, rust and e2e against the prebuilt image, skips contract, and passes. Recorded: 65/65, contract skipped, upgrade check skipped (no key), 4m47s in all (a first try failed only because the laptop slept mid-run).
- [x] 2.3 `.github/workflows/ci.yml` sets up the e2e dependencies, browsers and cached image build, runs `scripts/run-tests.sh --ci` and uploads the results on failure; on GitHub it passes on this branch's pull request (needs the branch pushed). Recorded: run 40512615488, test job passed in 14m12s (cold image build 5m25s, web 1m04s, Rust 54s, end-to-end 5m26s).

## 3. Infrastructure adapters (TDD)

None: no Rust changes beyond the `#[ignore]` reasons.

## 4. Verification

- [x] 4.1 `doc/DEVELOPMENT.md` (the main idea: one script, `--ci`, `--only`, CI runs everything but the contract tests) and `e2e-tests/README.md` updated; `scripts/run-tests.sh` passes locally. Recorded: `scripts/run-tests.sh` passed locally in 5m43s (web 422, Rust 1189, contract 16, upgrade check, end-to-end 65/65).
