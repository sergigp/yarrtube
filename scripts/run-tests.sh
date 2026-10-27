#!/usr/bin/env bash
# Runs every test suite: the web UI's checks, Rust formatting, lints and
# tests, the YouTube contract tests and the Playwright smoke suite. Used the
# same way locally and in CI.
#
# Usage: scripts/run-tests.sh [--ci]
#
# In CI (`--ci`, or `CI=true`, which GitHub Actions sets), the YouTube
# contract tests are skipped (YouTube bot-checks CI runners), and so, for
# now, is the smoke suite, until it is set up to run in CI.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

CI_MODE=false
if [[ "${CI:-}" == "true" ]]; then
  CI_MODE=true
fi
while [[ $# -gt 0 ]]; do
  case "$1" in
    --ci)
      CI_MODE=true
      shift
      ;;
    -h | --help)
      sed -n '2,11p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

step() {
  echo
  echo "==> $*"
}

skip() {
  echo
  echo "==> skipped: $*"
}

cd "$REPO_ROOT"

# Only checks: nothing is installed or built here. The web UI's dependencies
# must be installed and the web UI built (the Rust binary embeds it).
if [[ ! -d web/node_modules || ! -d web/dist ]]; then
  echo "error: run 'npm --prefix web ci && npm --prefix web run build' first" >&2
  exit 1
fi

step "web UI: typecheck, lint, test"
npm --prefix web run check

step "Rust: formatting"
cargo fmt --all -- --check

step "Rust: clippy"
cargo clippy --all-targets --all-features --locked -- -D warnings

step "Rust: tests"
cargo test --locked

if [[ "$CI_MODE" == true ]]; then
  skip "YouTube contract tests (they need the real YouTube)"
  skip "smoke suite (not set up for CI yet)"
else
  step "YouTube contract tests"
  "$REPO_ROOT/scripts/run-youtube-contract-tests.sh"

  step "smoke suite"
  "$REPO_ROOT/scripts/run-smoke-tests.sh"
fi

step "all tests passed"
