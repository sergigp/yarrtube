## 1. Walking skeleton

- [ ] 1.1 Rename everything: `git mv smoke-tests acceptance-tests`, `git mv scripts/run-smoke-tests.sh scripts/run-acceptance-tests.sh`, `git mv .github/workflows/smoke-tests.yml .github/workflows/acceptance-tests.yml`; rename the `SMOKE_*` variables, "smoke" wording, fixture ids and paths everywhere listed in design.md, and drop the unused variables. Done when `git grep -i smoke` finds nothing outside archived changes and `npx --prefix acceptance-tests playwright test --list` lists 66 tests.

## 2. Behaviour (TDD)

- [ ] 2.1 `scripts/run-acceptance-tests.sh` (no isolation yet) passes: 65 tests, routes covered.
- [ ] 2.2 Isolated runs: per-run container name and image tag, free ports (overridable), the failure log in `test-results/`, the image tag removed on exit. Two worktrees of this branch run `scripts/run-acceptance-tests.sh` at the same time and both pass.

## 3. Infrastructure adapters (TDD)

None: no Rust changes.

## 4. Verification

- [ ] 4.1 `scripts/run-tests.sh` passes (the acceptance suite with the upgrade check). Docs (`doc/DEVELOPMENT.md`, `acceptance-tests/README.md`) describe the new names and isolated runs.
