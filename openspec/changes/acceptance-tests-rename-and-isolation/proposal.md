## Why

Since `acceptance-tests-fake-youtube`, the smoke suite runs against a fake YouTube: it checks the app's behaviour against a controlled world, so it is an acceptance suite, and "smoke" (a quick end-to-end check against the real thing) no longer describes it. Separately, two runs can't happen at once: `scripts/run-smoke-tests.sh` force-removes a fixed `yarrtube-smoke` container, rebuilds a fixed `yarrtube:smoke` tag and binds fixed ports (8080, 18324, 18326), so a run in one worktree kills a run in another (this happened twice while implementing `smoke-tests-mobile-and-confidence`).

## What Changes

- **BREAKING** (developer tooling only): rename `smoke-tests/` → `acceptance-tests/`, `scripts/run-smoke-tests.sh` → `scripts/run-acceptance-tests.sh`, `.github/workflows/smoke-tests.yml` → `acceptance-tests.yml`, and every `SMOKE_*` variable → `ACCEPTANCE_*`, with no aliases; drop the variables nothing reads any more. Update the references in `scripts/run-tests.sh`, `CLAUDE.md`, `doc/DEVELOPMENT.md`, `doc/ARCHITECTURE.md`, `.claude/skills/rust-architect/SKILL.md`, `.claude/commands/audit-change.md`, `openspec/config.yaml` and `.gitignore`.
- Isolate each run: the container name and image tag carry a per-run id, the daemon and both stubs listen on free ports unless overridden, and the failure log goes in the suite's `test-results/`, so runs from different worktrees don't touch each other.
- No test behaviour changes.

## Capabilities

### New Capabilities
- `acceptance-tests`: the suite's requirements, moved over from `smoke-tests` under the new name, plus a requirement that runs from different worktrees are isolated from each other.

### Modified Capabilities
- `smoke-tests`: removed (its requirements move to `acceptance-tests`).

## Impact

- `smoke-tests/` (moved), `scripts/run-smoke-tests.sh` (moved), `.github/workflows/smoke-tests.yml` (moved), `scripts/run-tests.sh`, the docs and agent instructions listed above.
- Developers' local env vars for the suite.
- Comes before `acceptance-tests-parallel-and-ci`, which builds on the new names and on isolated runs.
