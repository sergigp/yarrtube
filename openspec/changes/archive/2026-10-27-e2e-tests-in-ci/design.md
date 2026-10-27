## Files

- `scripts/run-tests.sh`: the one test script. Suites `web`, `rust`, `contract`, `e2e`, in that order; `--only <suite>` (repeatable) runs just those; `--ci` (or `CI=true`) leaves out `contract`. Takes in `run-youtube-contract-tests.sh` (as `contract`) and `run-e2e-tests.sh` (as `e2e`, run in a subshell so its cleanup trap stays its own). Loads `.env` once. Checks each selected suite's prerequisites and installs nothing; `e2e` no longer runs `npm ci` or `playwright install`. `E2E_IMAGE` makes `e2e` use a prebuilt image instead of building one (and not remove it).
- `scripts/run-e2e-tests.sh`, `scripts/run-youtube-contract-tests.sh`: removed.
- `.github/workflows/ci.yml`: installs the end-to-end dependencies and browsers, builds the image under test with Buildx and the GitHub Actions cache (`load: true`, tag `yarrtube:e2e-ci`), runs `scripts/run-tests.sh --ci` with `E2E_IMAGE=yarrtube:e2e-ci`, and uploads `e2e-tests/test-results/` (which holds the container log) on failure.
- `.github/workflows/e2e-tests.yml`: removed.
- `doc/DEVELOPMENT.md`, `e2e-tests/README.md`, `CLAUDE.md`, `openspec/config.yaml`, the e2e helpers' comments, the contract tests' `#[ignore]` reasons and `youtube_contract.rs`: `scripts/run-tests.sh --only e2e` / `--only contract` instead of the removed scripts.

## Types & Signatures

```bash
scripts/run-tests.sh [--ci] [--only web|rust|contract|e2e]...
E2E_IMAGE=<image>   # e2e: test this image instead of building one
```

## Call Stack

- CI: `ci.yml` → install web deps + build web UI → install e2e deps + browsers → `docker/build-push-action` (gha cache) → `E2E_IMAGE=yarrtube:e2e-ci scripts/run-tests.sh --ci` → web, rust, (contract skipped), e2e → on failure upload `e2e-tests/test-results/`.
- Local: `scripts/run-tests.sh` → web, rust, contract, e2e (builds its own per-run image); `scripts/run-tests.sh --only e2e` → just the end-to-end suite.

## Test Plan

No application behaviour changes, so no Rust tests; the scripts and CI are checked by running them:

1. Behaviour:
   1. `scripts/run-tests.sh --only contract` and `--only e2e` each run just that suite and pass.
   2. `CI=true E2E_IMAGE=<prebuilt> scripts/run-tests.sh` runs web, rust and e2e (contract skipped) against the prebuilt image and passes.
   3. On GitHub, the CI workflow runs the end-to-end suite on this branch's pull request and passes.
