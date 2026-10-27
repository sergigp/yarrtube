## Why

"Smoke" (a quick check that the thing basically works) no longer describes the suite: it is the project's full end-to-end layer, driving the real binary in its Docker image through real browsers, with only YouTube faked (since `acceptance-tests-fake-youtube`), like any external dependency. "Acceptance" was considered and rejected: the project's Rust conventions already call the handler-level tests acceptance tests. Separately, two runs can't happen at once: `scripts/run-smoke-tests.sh` force-removes a fixed `yarrtube-smoke` container, rebuilds a fixed `yarrtube:smoke` tag and binds fixed ports (8080, 18324, 18326), so a run in one worktree kills a run in another (this happened twice while implementing `smoke-tests-mobile-and-confidence`).

## What Changes

- **BREAKING** (developer tooling only): rename `smoke-tests/` → `e2e-tests/`, `scripts/run-smoke-tests.sh` → `scripts/run-e2e-tests.sh`, `.github/workflows/smoke-tests.yml` → `e2e-tests.yml`, and every `SMOKE_*` variable → `E2E_*`, with no aliases; drop the variables nothing reads any more. Update the references in `scripts/run-tests.sh`, `CLAUDE.md`, `doc/DEVELOPMENT.md`, `doc/ARCHITECTURE.md`, `.claude/skills/rust-architect/SKILL.md`, `openspec/config.yaml` and `.gitignore`.
- Isolate each run: the container name and image tag carry a per-run id, the daemon and both stubs listen on free ports unless overridden, and the failure log goes in the suite's `test-results/`, so runs from different worktrees don't touch each other.
- No test behaviour changes.

## Capabilities

### New Capabilities
- `e2e-tests`: the suite's requirements, moved over from `smoke-tests` under the new name, plus a requirement that runs from different worktrees are isolated from each other.

### Modified Capabilities
- `smoke-tests`: removed (its requirements move to `e2e-tests`).

## Impact

- `smoke-tests/` (moved), `scripts/run-smoke-tests.sh` (moved), `.github/workflows/smoke-tests.yml` (moved), `scripts/run-tests.sh`, the docs and agent instructions listed above.
- Developers' local env vars for the suite.
- Comes before `e2e-tests-parallel-and-ci`, which builds on the new names and on isolated runs.
