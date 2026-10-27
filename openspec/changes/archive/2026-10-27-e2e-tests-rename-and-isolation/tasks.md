## 1. Walking skeleton

- [x] 1.1 Rename everything: `git mv smoke-tests e2e-tests`, `git mv scripts/run-smoke-tests.sh scripts/run-e2e-tests.sh`, `git mv .github/workflows/smoke-tests.yml .github/workflows/e2e-tests.yml`; rename the `SMOKE_*` variables to `E2E_*`, the "smoke" wording, fixture ids and paths everywhere listed in design.md, and drop the unused variables. Done when `git grep -i smoke` finds nothing outside archived changes and the project-agnostic `.claude/commands/audit-change.md`, and `npx --prefix e2e-tests playwright test --list` lists 65 tests.

## 2. Behaviour (TDD)

- [x] 2.1 `scripts/run-e2e-tests.sh` (no isolation yet) passes: 65 tests, routes covered.
- [x] 2.2 Isolated runs: per-run container name and image tag, free ports (overridable), the failure log in `test-results/`, the image tag removed on exit. Two worktrees of this branch run `scripts/run-e2e-tests.sh` at the same time and both pass.

## 3. Infrastructure adapters (TDD)

None: no Rust changes.

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh` passes (the end-to-end suite with the upgrade check). Docs (`doc/DEVELOPMENT.md`, `e2e-tests/README.md`) describe the new names and isolated runs. Recorded 2026-10-27: `scripts/run-tests.sh` passed in 4m47s (web 422, Rust 1189, contract 16, upgrade check, end-to-end 65/65 in 3.5m); two worktrees ran `scripts/run-e2e-tests.sh` at once, both 65/65 in ~4.6 min.
