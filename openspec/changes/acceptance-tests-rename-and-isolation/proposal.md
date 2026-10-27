## Why

The smoke suite is about to stop being end to end (YouTube will be faked, see `acceptance-tests-fake-youtube`), so "smoke" no longer describes it: it checks the app's behaviour against a controlled world, i.e. acceptance tests. Separately, two runs can't happen at once: `scripts/run-smoke-tests.sh` force-removes a fixed `yarrtube-smoke` container, binds port 8080, rebuilds a fixed `yarrtube:smoke` tag and uses Plex stub port 18324, so a run in one worktree kills a run in another (this happened twice while implementing `smoke-tests-mobile-and-confidence`).

## What Changes

- **BREAKING** (developer tooling only): rename `smoke-tests/` → `acceptance-tests/`, `scripts/run-smoke-tests.sh` → `scripts/run-acceptance-tests.sh`, `.github/workflows/smoke-tests.yml` → `acceptance-tests.yml`, and every `SMOKE_*` variable → `ACCEPTANCE_*`, with no aliases. Update the references in `CLAUDE.md`, `doc/DEVELOPMENT.md`, `doc/ARCHITECTURE.md`, `.claude/skills/rust-architect/SKILL.md`, `.claude/commands/audit-change.md` and `openspec/config.yaml`.
- Isolate each run: derive the container name and image tag from a per-run id, pick free ports for the daemon and the Plex stub, and keep the failure log in the run's temp directory, so runs from different worktrees (or two in one) don't touch each other.
- No test behaviour changes.

## Capabilities

### New Capabilities
- `acceptance-tests`: the suite's requirements, moved over from `smoke-tests` under the new name, plus a requirement that concurrent runs are isolated from each other.

### Modified Capabilities
- `smoke-tests`: removed (its requirements move to `acceptance-tests`).

## Impact

- `smoke-tests/` (moved), `scripts/run-smoke-tests.sh` (moved), `.github/workflows/smoke-tests.yml` (moved), docs and agent instructions listed above.
- Developers' local env vars and any muscle memory for `run-smoke-tests.sh`.
- Comes first: the later phases build on the new names.
