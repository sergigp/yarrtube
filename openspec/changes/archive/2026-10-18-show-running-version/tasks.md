# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md and wire it end to end: `APP_VERSION` in `infrastructure/shared/app_version.rs` (hardcoded `"dev"`), `RunningVersion` + `running_version` in `ApiServices`, `GET /version` route to `get_version` (returns a fixed `VersionResponse { version: "" }`), `RunningVersion(APP_VERSION.to_string())` built in `serve.rs`; `AppVersion` type, `fetchVersion`, `useVersion`, `VersionLabel` (renders `null`) placed after the logo link in `App.tsx`; `GET /api/version` routed in `AppShell.test.tsx`'s `renderShell`; `ARG YARRTUBE_VERSION` in the Dockerfile build stage right before `cargo build --release`; `release.yml` build job computes `YARRTUBE_VERSION` (`inputs.tag` / `edge-<short sha>` on `workflow_dispatch` / `github.ref_name`) and passes it as a build arg. Done when `cargo build` and `npm run build` succeed and `cargo test --locked` and `npm run check` pass with no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_return_the_running_version` — `get_version` returns the version held in `RunningVersion`. Verify: `cargo test it_should_return_the_running_version`.
- [x] 2.2 `shows the running version next to the logo` — header shows exactly `v0.9.2` when `GET /api/version` returns it. Verify: `npm run test -- AppShell`.
- [x] 2.3 `stays on the current view when the version is clicked` — clicking the version on `/tasks` keeps the tasks view. Verify: `npm run test -- AppShell`.
- [x] 2.4 `shows no version while it is loading` — nothing rendered while the request is pending. Verify: `npm run test -- AppShell`.
- [x] 2.5 `shows no version and no error when the request fails` — no version, no error text, the header still renders. Verify: `npm run test -- AppShell`.

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `it_should_report_dev_without_a_build_version` — replace the hardcoded `APP_VERSION` with `option_env!("YARRTUBE_VERSION")`, falling back to `dev`. Verify: `cargo test it_should_report_dev_without_a_build_version`.
- [x] 3.2 Smoke spec `shows the running version in the header` in `smoke-tests/tests/version.spec.js` — the header shows `dev` against the local binary (covers the `/api/version` route wiring). Verify: `scripts/run-smoke-tests.sh`.

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `npm run check` (in `web/`) all pass.
- [x] 4.2 `scripts/run-smoke-tests.sh` passes.
- [x] 4.3 Manual check: `scripts/run-local.sh` shows `dev` next to the logo on desktop and mobile widths; `docker build --build-arg YARRTUBE_VERSION=v0.0.0-test .` produces an image whose `GET /api/version` returns `{"version":"v0.0.0-test"}` and whose header shows `v0.0.0-test`.
