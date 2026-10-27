## Files

- `smoke-tests/` → `acceptance-tests/` (`git mv`): every file, with "smoke" wording and `SMOKE_*` variables renamed inside (README, `playwright.config.js` errors, `package.json` name, stubs, helpers, specs, fixture ids like `smoke-a`, the Plex stub token `smoke-token`).
- `scripts/run-smoke-tests.sh` → `scripts/run-acceptance-tests.sh`: renamed variables; per-run container name and image tag; free ports for the daemon and both stubs unless `ACCEPTANCE_PORT`, `ACCEPTANCE_PLEX_STUB_PORT`, `ACCEPTANCE_YOUTUBE_STUB_PORT` are set; the container log on failure at `acceptance-tests/test-results/container.log`; removes its image tag on exit. Drops `SMOKE_CHANNEL_HANDLE` and `SMOKE_CHANNEL_VIDEO_LIMIT`, which nothing reads since the fake YouTube.
- `.github/workflows/smoke-tests.yml` → `acceptance-tests.yml`: the new script and names; still manual (CI comes with `acceptance-tests-parallel-and-ci`).
- `scripts/run-tests.sh`: calls `run-acceptance-tests.sh`; step names.
- `CLAUDE.md`, `doc/DEVELOPMENT.md`, `doc/ARCHITECTURE.md`, `.claude/skills/rust-architect/SKILL.md`, `.claude/commands/audit-change.md`, `openspec/config.yaml`, `.gitignore`: the new names.

## Types & Signatures

```bash
# scripts/run-acceptance-tests.sh
RUN_ID="$(date +%s)-$$"
CONTAINER_NAME="yarrtube-acceptance-$RUN_ID"
IMAGE_TAG="yarrtube:acceptance-$RUN_ID"
free_port()            # prints a free TCP port on localhost (via node, which the suite needs anyway)
PORT="${ACCEPTANCE_PORT:-$(free_port)}"
PLEX_STUB_PORT="${ACCEPTANCE_PLEX_STUB_PORT:-$(free_port)}"
YOUTUBE_STUB_PORT="${ACCEPTANCE_YOUTUBE_STUB_PORT:-$(free_port)}"
LOG_FILE="$REPO_ROOT/acceptance-tests/test-results/container.log"
```

## Call Stack

- `scripts/run-tests.sh` → `scripts/run-acceptance-tests.sh` → `docker build -t yarrtube:acceptance-<run id>` → stubs on their ports → `docker run --name yarrtube-acceptance-<run id> -p <port>:<port> -e YARRTUBE_PORT=<port> …` → `npm --prefix acceptance-tests test` with `BASE_URL=http://localhost:<port>`, `ACCEPTANCE_PLEX_STUB_URL`, `ACCEPTANCE_YOUTUBE_STUB_PORT`, … → on exit: `docker rm -f <container>`, `docker image rm <tag>`, stubs killed.

## Test Plan

No application or adapter behaviour changes, so there are no Rust tests. The suite itself is the check:

1. Behaviour (the suite):
   1. After the rename, `scripts/run-acceptance-tests.sh` passes as `run-smoke-tests.sh` did (65 tests).
   2. Two worktrees of this branch run `scripts/run-acceptance-tests.sh` at the same time (upgrade check skipped): both pass, each with its own container and ports.
