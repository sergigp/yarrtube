## Why

The end-to-end suite no longer talks to YouTube (since `acceptance-tests-fake-youtube`), so nothing keeps it out of CI any more, yet `scripts/run-tests.sh --ci` still skips it and its own workflow is manual-only. Testing is also spread over three scripts (`run-tests.sh`, `run-e2e-tests.sh`, `run-youtube-contract-tests.sh`); one entry point is enough.

## What Changes

- CI runs the end-to-end suite on every pull request and push to `main`, as part of `scripts/run-tests.sh --ci`, without a YouTube API key (the upgrade check, which needs the real YouTube, skips itself). The image under test is built by the workflow with Docker layer caching and handed to the suite; a failing run keeps its Playwright results and container log as artifacts.
- `scripts/run-tests.sh` becomes the only test script: it takes in the end-to-end runner and the YouTube contract runner, keeps `--ci` (which skips only the contract tests now), and gains `--only <suite>` (`web`, `rust`, `contract`, `e2e`) to run one suite. It still only checks: dependencies, browsers and the web build are prerequisites (CI installs them in its own steps). `scripts/run-e2e-tests.sh` and `scripts/run-youtube-contract-tests.sh` go; `scripts/` holds `run-tests.sh` and `run-local.sh`.
- The manual `.github/workflows/e2e-tests.yml` goes: CI covers it, and its one extra, the upgrade check against the real YouTube, is bot-checked on GitHub's runners.
- Parallel workers (a daemon per Playwright worker) are left for a later change.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `e2e-tests`: the "Continuous Integration" requirement: on every pull request and push to `main`, without a YouTube API key, with failure artifacts.

## Impact

- `scripts/run-tests.sh` (takes in the two runners), `scripts/run-e2e-tests.sh` and `scripts/run-youtube-contract-tests.sh` (removed), `.github/workflows/ci.yml`, `.github/workflows/e2e-tests.yml` (removed).
- References to the removed scripts: `doc/DEVELOPMENT.md`, `e2e-tests/README.md`, `e2e-tests/playwright.config.js` and the routes reporter, the contract tests' `#[ignore]` reasons and `src/infrastructure/shared/youtube_contract.rs`, `CLAUDE.md`.
- CI takes longer (the image build, cached between runs, and ~4 min of end-to-end tests); publishing `:edge` already waits for the CI job.
