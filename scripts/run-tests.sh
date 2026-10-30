#!/usr/bin/env bash
# Runs the test suites, the same way locally and in CI:
#   web       the web UI's typecheck, lint and tests
#   rust      Rust formatting, clippy and tests
#   contract  the YouTube contract tests: adapters against the real yt-dlp
#             and YouTube Data API (needs yt-dlp, ffprobe, YOUTUBE_API_KEY)
#   e2e       the end-to-end suite: the Docker image driven through real
#             browsers against a fake YouTube (needs Docker)
#
# Usage: scripts/run-tests.sh [--ci] [--workers <N>] [--only <suite>]...
#
# `--only` (repeatable) runs just those suites. `--workers` sets how many
# end-to-end tests run at once, each worker against its own daemon
# (default 3). In CI (`--ci`, or `CI=true`, which GitHub Actions sets) the
# contract tests are left out (YouTube bot-checks CI runners) and failing
# end-to-end tests are retried. It only runs checks: dependencies, the
# Playwright browsers and the web UI build must already be in place, and are
# checked before anything runs. It first prints what will run and how. Each
# phase's output goes to a log, not the terminal:
# a progress line while it runs, its time and tests after, and when it fails
# just the failing tests (the logs are kept, their directory printed).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ALL_SUITES=(web rust contract e2e)

CI_MODE=false
if [[ "${CI:-}" == "true" ]]; then
  CI_MODE=true
fi
ONLY=()
WORKERS=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --ci)
      CI_MODE=true
      shift
      ;;
    --only)
      if [[ ! " ${ALL_SUITES[*]} " == *" ${2:-} "* ]]; then
        echo "unknown suite: ${2:-} (one of: ${ALL_SUITES[*]})" >&2
        exit 1
      fi
      ONLY+=("$2")
      shift 2
      ;;
    --workers)
      if [[ -z "${2:-}" ]]; then
        echo "--workers takes a positive number" >&2
        exit 1
      fi
      WORKERS="$2"
      shift 2
      ;;
    -h | --help)
      sed -n '2,21p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

# End-to-end workers: 3 unless asked otherwise, fast on a laptop and on
# GitHub's 4-vCPU runners alike.
WORKERS="${WORKERS:-3}"
if [[ ! "$WORKERS" =~ ^[1-9][0-9]*$ ]]; then
  echo "--workers takes a positive number (got \"$WORKERS\")" >&2
  exit 1
fi

# CI mode also makes Playwright retry a failing end-to-end test (see
# e2e-tests/playwright.config.js), which reads CI.
if [[ "$CI_MODE" == true ]]; then
  export CI=true
fi

if [[ -f "$REPO_ROOT/.env" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
  set +a
fi

step() {
  echo
  echo "==> $*"
}

# Every phase's output, kept when the run fails.
LOG_DIR="$(mktemp -d "${TMPDIR:-/tmp}/yarrtube-tests.XXXXXX")"
SCRIPT_START=$(date +%s)
INTERACTIVE=false
if [[ -t 1 ]]; then
  INTERACTIVE=true
fi

SUITE=""
PHASE_NAME=""
PHASE_KIND=""
PHASE_LOG=""
PHASE_START=0
TICKER_PID=""

# Runs a phase: `run_phase <name> <kind> <command>...`, the command's output
# going to the phase's log. The kind (vitest, cargo, cargo-build, playwright,
# docker, plain) says how to show its progress and pick out its tests. A
# failing command exits the script, whose EXIT trap reports the phase still
# open as the failed one. The command runs in this shell, so the state it
# sets (e.g. CONTAINER_STARTED) is seen by the cleanup.
run_phase() {
  PHASE_NAME="$1"
  PHASE_KIND="$2"
  shift 2
  PHASE_LOG="$LOG_DIR/$(tr -cs 'A-Za-z0-9' '-' <<<"$SUITE $PHASE_NAME" | sed 's/-$//').log"
  PHASE_START=$(date +%s)
  if [[ "$INTERACTIVE" == true ]]; then
    ticker &
    TICKER_PID=$!
  else
    echo "  … $PHASE_NAME"
  fi
  "$@" >"$PHASE_LOG" 2>&1
  stop_ticker
  phase_line "✓" "$(phase_tests)"
  PHASE_NAME=""
}

# Reports the open phase, if any, as failed, with its failing tests.
phase_fail_open() {
  if [[ -z "$PHASE_NAME" ]]; then
    return
  fi
  stop_ticker
  phase_line "✗" "FAILED"
  print_failures
  echo "    full output: $PHASE_LOG"
  PHASE_NAME=""
}

phase_line() {
  local mark="$1" detail="$2"
  if [[ "$INTERACTIVE" == true ]]; then
    case "$mark" in
      ✓) mark=$'\033[32m✓\033[0m' ;;
      ✗) mark=$'\033[31m✗\033[0m' ;;
    esac
  fi
  printf '  %s %-24s %7s  %s\n' "$mark" "$PHASE_NAME" "$(duration $(( $(date +%s) - PHASE_START )))" "$detail"
}

# The progress line, redrawn while a phase runs on a terminal.
ticker() {
  set +e
  local frames=(⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏) i=0
  while true; do
    printf '\r\033[K  %s %-24s %7s  %s' "${frames[i++ % 10]}" "$PHASE_NAME" \
      "$(duration $(( $(date +%s) - PHASE_START )))" "$(phase_progress)"
    sleep 0.5
  done
}

stop_ticker() {
  if [[ -n "$TICKER_PID" ]]; then
    kill "$TICKER_PID" 2>/dev/null || true
    wait "$TICKER_PID" 2>/dev/null || true
    TICKER_PID=""
    printf '\r\033[K'
  fi
}

strip_colors() {
  sed $'s/\x1b\\[[0-9;]*[A-Za-z]//g' "$@"
}

# How far the open phase got, from its log so far.
phase_progress() {
  case "$PHASE_KIND" in
    vitest)
      strip_colors "$PHASE_LOG" | awk '/^ *[✓❯×↓] .*\([0-9]+ tests?/ {
          match($0, /\([0-9]+ tests?/); n += substr($0, RSTART + 1, RLENGTH - 1) + 0
        }
        END { if (n) printf "%d tests", n }'
      ;;
    cargo | cargo-build)
      local finished
      finished=$(grep -cE '^test .* \.\.\. ' "$PHASE_LOG")
      if (( finished > 0 )); then
        echo "$finished tests"
      elif grep -qE '^ *(Compiling|Checking) ' "$PHASE_LOG"; then
        echo "compiling"
      fi
      ;;
    playwright)
      strip_colors "$PHASE_LOG" | awk '
        /Running [0-9]+ tests? using/ { total += $2 }
        /^ +[✓✘-] +[0-9]+ / { done++ }
        END { if (total) printf "%d/%d tests", done, total }'
      ;;
    docker)
      strip_colors "$PHASE_LOG" | grep -oE '\[[^]]+ [0-9]+/[0-9]+\]' | tail -1
      ;;
  esac
}

# The open phase's test counts, from its finished log.
phase_tests() {
  case "$PHASE_KIND" in
    vitest) count_vitest "$PHASE_LOG" ;;
    cargo) count_cargo "$PHASE_LOG" ;;
    playwright) count_playwright "$PHASE_LOG" ;;
  esac
}

# "426 passed" from Vitest's closing "Tests  426 passed (426)".
count_vitest() {
  strip_colors "$1" | sed -nE 's/^ *Tests +(.*) \([0-9]+\) *$/\1/p' | tail -1
}

# Every cargo test binary's "test result:" line's passed tests, added up.
# The ignored ones aren't counted: they're the contract tests, a suite of
# their own.
count_cargo() {
  strip_colors "$1" | awk '/^test result:/ {
      for (i = 1; i <= NF; i++) if ($(i + 1) ~ /^passed/) passed += $i
    }
    END { printf "%d passed", passed }'
}

# Playwright's closing counts ("65 passed (1.4m)", "1 flaky", ...) of every
# run in the log, added up per kind.
count_playwright() {
  strip_colors "$@" | awk '/^ +[0-9]+ (passed|failed|flaky|skipped|did not run)/ {
      n = $1; $1 = ""; sub(/^ /, ""); sub(/ \(.*\)$/, ""); count[$0] += n
      if (!($0 in seen)) { seen[$0] = 1; order[++kinds] = $0 }
    }
    END { for (i = 1; i <= kinds; i++) printf "%s%d %s", (i > 1 ? ", " : ""), count[order[i]], order[i] }'
}

# The failed phase's failing tests, each with its error's first lines. When
# none can be picked out (a compile error, lint, a container that never got
# ready...), the log from its first error, or its tail.
print_failures() {
  local failures=""
  case "$PHASE_KIND" in
    vitest)
      failures=$(strip_colors "$PHASE_LOG" | awk '
        /^ *FAIL / { sub(/^ *FAIL +/, ""); print "    ✗ " $0; want = 1; next }
        want && NF && !/⎯/ { sub(/^ +/, ""); print "        " $0; want = 0 }')
      ;;
    cargo)
      failures=$(strip_colors "$PHASE_LOG" | awk '
        /^---- .* stdout ----$/ { print "    ✗ " $2; block = 1; lines = -1; next }
        /^failures:$/ { block = 0 }
        block && /panicked at|^Error: / { lines = 0 }
        block && lines >= 0 && lines < 8 && NF && !/^note: run with/ { print "        " $0; lines++ }')
      ;;
    playwright)
      failures=$(strip_colors "$PHASE_LOG" | awk '
        /^ +[0-9]+\) \[/ { sub(/^ +[0-9]+\) /, ""); sub(/ *─+ *$/, ""); print "    ✗ " $0; want = 1; next }
        want && /Error/ { sub(/^ +/, ""); print "        " $0; want = 0 }')
      ;;
  esac
  if [[ -n "$failures" ]]; then
    echo "$failures"
    return
  fi
  local cleaned
  cleaned=$(strip_colors "$PHASE_LOG" | grep -vE '^ *(Compiling|Checking|Downloading|Downloaded|Blocking|Updating|Locking|Finished) ')
  if grep -qE '^error' <<<"$cleaned"; then
    sed -n '/^error/,$p' <<<"$cleaned" | head -60 | sed 's/^/    /'
  else
    tail -40 <<<"$cleaned" | sed 's/^/    /'
  fi
}

duration() {
  local seconds="$1"
  if (( seconds < 60 )); then
    echo "${seconds}s"
  else
    printf '%dm%02ds\n' $(( seconds / 60 )) $(( seconds % 60 ))
  fi
}

finish() {
  local exit_code=$?
  phase_fail_open
  local total
  total=$(duration $(( $(date +%s) - SCRIPT_START )))
  if [[ $exit_code -eq 0 ]]; then
    rm -rf "$LOG_DIR"
    step "all selected suites passed ($total)"
  elif [[ -z "$(ls -A "$LOG_DIR")" ]]; then
    rm -rf "$LOG_DIR"
  else
    step "failed (exit $exit_code, $total); logs in $LOG_DIR"
  fi
  exit $exit_code
}
trap finish EXIT

selected() {
  local suite="$1"
  if [[ ${#ONLY[@]} -gt 0 ]]; then
    [[ " ${ONLY[*]} " == *" $suite "* ]]
  else
    [[ "$suite" != contract || "$CI_MODE" == false ]]
  fi
}

# The contract tests' yt-dlp.
YTDLP="${YTDLP_PATH:-$(command -v yt-dlp || true)}"
# The image the end-to-end upgrade check starts from: the latest release, so
# every change is checked against the data the version users run today
# leaves behind. `none` skips the check. Only the upgrade check needs the
# real YouTube, since the released image it starts can't be pointed at the
# fake one: it runs only with a real YOUTUBE_API_KEY.
UPGRADE_FROM_IMAGE="${E2E_UPGRADE_FROM_IMAGE:-ghcr.io/sergigp/yarrtube:latest}"
UPGRADE_SKIPPED_BECAUSE=""
if [[ "$UPGRADE_FROM_IMAGE" == "none" ]]; then
  UPGRADE_SKIPPED_BECAUSE="E2E_UPGRADE_FROM_IMAGE=none"
elif [[ -z "${YOUTUBE_API_KEY:-}" ]]; then
  UPGRADE_SKIPPED_BECAUSE="no YOUTUBE_API_KEY, it needs the real YouTube"
  UPGRADE_FROM_IMAGE="none"
fi

# What the selected suites need, all checked before any runs, so a missing
# one doesn't fail the run halfway. Every problem is listed at once.
check_dependencies() {
  local missing=()
  need_dir() {
    if [[ ! -d "$REPO_ROOT/$1" ]]; then
      missing+=("$1 is missing: run '$2'")
    fi
  }
  need_command() {
    if ! command -v "$1" >/dev/null; then
      missing+=("$1 not found on PATH${2:+ ($2)}")
    fi
  }
  if selected web; then
    need_command npm
    need_dir web/node_modules "npm --prefix web ci"
  fi
  if selected rust || selected contract; then
    need_command cargo
    # The Rust binary embeds the built web UI.
    need_dir web/dist "npm --prefix web ci && npm --prefix web run build"
  fi
  if selected contract; then
    if [[ -z "${YOUTUBE_API_KEY:-}" ]]; then
      missing+=("YOUTUBE_API_KEY is not set (in the environment or .env)")
    fi
    if [[ -z "$YTDLP" ]]; then
      missing+=("yt-dlp not found on PATH (or set YTDLP_PATH)")
    fi
    need_command ffprobe "it comes with ffmpeg, which yt-dlp also needs"
  fi
  if selected e2e; then
    need_command node
    need_dir e2e-tests/node_modules \
      "npm --prefix e2e-tests ci && npx --prefix e2e-tests playwright install --with-deps chromium webkit"
    if ! command -v docker >/dev/null; then
      missing+=("docker not found on PATH")
    elif ! docker info >/dev/null 2>&1; then
      missing+=("the Docker daemon isn't running")
    fi
  fi
  if [[ ${#missing[@]} -gt 0 ]]; then
    step "missing dependencies"
    printf '  ✗ %s\n' "${missing[@]}"
    exit 1
  fi
}

# What is about to run, and how.
print_plan() {
  local run=() skipped=() suite
  for suite in "${ALL_SUITES[@]}"; do
    if selected "$suite"; then run+=("$suite"); else skipped+=("$suite"); fi
  done
  step "plan"
  printf '  %-10s %s\n' "suites" "${run[*]}${skipped[*]:+ (skipped: ${skipped[*]})}"
  if [[ "$CI_MODE" == true ]]; then
    printf '  %-10s %s\n' "CI mode" "on: contract tests left out unless --only names them" \
      "" "(YouTube bot-checks CI runners), failing e2e tests retried twice"
  else
    printf '  %-10s %s\n' "CI mode" "off (--ci or CI=true: no contract tests, e2e retries)"
  fi
  if selected contract; then
    printf '  %-10s %s\n' "yt-dlp" "$("$YTDLP" --version) at $YTDLP"
  fi
  if selected e2e; then
    printf '  %-10s %s\n' "e2e" "$WORKERS workers, image ${E2E_IMAGE:-built from this tree}"
    if [[ -n "$UPGRADE_SKIPPED_BECAUSE" ]]; then
      printf '  %-10s %s\n' "upgrade" "skipped: $UPGRADE_SKIPPED_BECAUSE"
    else
      printf '  %-10s %s\n' "upgrade" "from $UPGRADE_FROM_IMAGE"
    fi
  fi
}

run_web() {
  step "web UI"
  run_phase "typecheck" plain npm --prefix "$REPO_ROOT/web" run typecheck
  run_phase "lint" plain npm --prefix "$REPO_ROOT/web" run lint
  # Vitest's default reporter lists each test file as it finishes (others,
  # picked when it detects an agent or CI, only the failing ones), which the
  # progress line counts.
  run_phase "tests" vitest npm --prefix "$REPO_ROOT/web" run test -- --reporter=default
}

run_rust() {
  step "Rust"
  run_phase "formatting" plain cargo fmt --all -- --check
  run_phase "clippy" cargo-build cargo clippy --all-targets --all-features --locked -- -D warnings
  run_phase "tests" cargo cargo test --locked
}

# The adapters' `#[ignore]`d tests against the real yt-dlp and YouTube Data
# API (modules named `youtube_contract_tests`). Run them after upgrading
# yt-dlp or when downloads start failing, to see whether YouTube's side
# changed.
run_contract() {
  step "YouTube contract tests"
  YOUTUBE_API_KEY="$YOUTUBE_API_KEY" YTDLP_PATH="$YTDLP" \
    run_phase "tests" cargo cargo test --locked -- --ignored youtube_contract
}

# Builds the release Docker image (or takes `E2E_IMAGE`), runs it with fresh
# temp database/videos directories against a fake YouTube, and drives it with
# the Playwright suite. Before that, the upgrade check: the released image
# builds up a library, then this build starts on the same database and videos
# and must carry it on as it was. Everything a run creates carries its own id
# and ports, so runs from different worktrees can go at the same time. Run in
# a subshell, so its cleanup trap is its own.
run_e2e() {
  step "end-to-end"

  local run_id
  run_id="$(date +%s)-$$"
  IMAGE_TAG="${E2E_IMAGE:-yarrtube:e2e-$run_id}"
  CONTAINER_NAME="yarrtube-e2e-$run_id"
  READY_TIMEOUT_SECONDS="${E2E_READY_TIMEOUT_SECONDS:-120}"
  # Free ports for the daemon and both stubs, unless set; asked for together
  # so the three can't collide.
  local free_port free_plex_stub_port free_youtube_stub_port
  read -r free_port free_plex_stub_port free_youtube_stub_port < <(node -e '
    const net = require("node:net")
    const servers = [0, 1, 2].map(() => net.createServer().listen(0, "127.0.0.1"))
    Promise.all(servers.map((s) => new Promise((resolve) => s.on("listening", resolve)))).then(() => {
      console.log(servers.map((s) => s.address().port).join(" "))
      servers.forEach((s) => s.close())
    })
  ')
  PORT="${E2E_PORT:-$free_port}"
  PLEX_STUB_PORT="${E2E_PLEX_STUB_PORT:-$free_plex_stub_port}"
  YOUTUBE_STUB_PORT="${E2E_YOUTUBE_STUB_PORT:-$free_youtube_stub_port}"
  # The upgrade check's container log, kept with this worktree's test results
  # when the check fails (each worker keeps its own daemon's, see daemon.js).
  LOG_FILE="$REPO_ROOT/e2e-tests/test-results/upgrade-container.log"

  # The suite runs against a fake YouTube (see e2e-tests/README.md).
  # The upgrade check's fixture: the maintainer-owned "test" playlist on the
  # real YouTube.
  E2E_PLAYLIST_ID="${E2E_PLAYLIST_ID:-PLXWRoRTUXjks}"
  E2E_PLAYLIST_NAME="${E2E_PLAYLIST_NAME:-test}"

  TMP_DIR=""
  CONTAINER_STARTED=false
  PLEX_STUB_PID=""
  YOUTUBE_STUB_PID=""
  trap e2e_cleanup EXIT

  if [[ -z "${E2E_IMAGE:-}" ]]; then
    run_phase "image build" docker docker build -t "$IMAGE_TAG" "$REPO_ROOT"
  fi

  # The fake YouTube Data API on the host, shared by every daemon (it only
  # reads); the stand-in yt-dlp is mounted into each container (see
  # e2e-tests/fake-youtube/).
  E2E_YOUTUBE_STUB_PORT="$YOUTUBE_STUB_PORT" \
    node "$REPO_ROOT/e2e-tests/fake-youtube/youtube-stub.js" >"$LOG_DIR/youtube-stub.log" 2>&1 &
  YOUTUBE_STUB_PID=$!

  if [[ "$UPGRADE_FROM_IMAGE" != "none" ]]; then
    TMP_DIR="$(mktemp -d /tmp/yarrtube-e2e.XXXXXX)"
    mkdir -p "$TMP_DIR/videos" "$TMP_DIR/config"
    # A stand-in Plex server on the host, so the check can see yarrtube asks
    # Plex to scan what it downloads (see e2e-tests/plex-stub.js).
    E2E_PLEX_STUB_PORT="$PLEX_STUB_PORT" \
      node "$REPO_ROOT/e2e-tests/plex-stub.js" >"$LOG_DIR/plex-stub.log" 2>&1 &
    PLEX_STUB_PID=$!
    run_phase "upgrade check" playwright e2e_upgrade_check
  fi

  # Each worker starts its own daemon from the image (see
  # e2e-tests/helpers/daemon.js).
  E2E_IMAGE="$IMAGE_TAG" \
    E2E_RUN_ID="$run_id" \
    E2E_YOUTUBE_STUB_PORT="$YOUTUBE_STUB_PORT" \
    E2E_WORKERS="$WORKERS" \
    E2E_CHECK_ROUTE_COVERAGE=1 \
    run_phase "suite ($WORKERS workers)" playwright npm --prefix "$REPO_ROOT/e2e-tests" test
}

# The released image seeds a library, then this build starts on it and must
# carry it on as it was.
e2e_upgrade_check() {
  echo "==> seeding a library with $UPGRADE_FROM_IMAGE"
  docker pull "$UPGRADE_FROM_IMAGE"
  e2e_start_container "$UPGRADE_FROM_IMAGE" real
  e2e_run_upgrade_phase seed
  e2e_stop_container

  echo "==> verifying the seeded library on $IMAGE_TAG"
  e2e_start_container "$IMAGE_TAG" fake
  e2e_run_upgrade_phase verify
  e2e_fail_on_panic
  e2e_stop_container
}

e2e_cleanup() {
  local exit_code=$?
  phase_fail_open
  if [[ "$CONTAINER_STARTED" == true ]]; then
    if [[ $exit_code -ne 0 ]]; then
      echo "    container logs: $LOG_FILE"
      mkdir -p "$(dirname "$LOG_FILE")"
      docker logs "$CONTAINER_NAME" >"$LOG_FILE" 2>&1 || true
    fi
    docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
  fi
  if [[ -n "$PLEX_STUB_PID" ]]; then
    kill "$PLEX_STUB_PID" 2>/dev/null || true
  fi
  if [[ -n "$YOUTUBE_STUB_PID" ]]; then
    kill "$YOUTUBE_STUB_PID" 2>/dev/null || true
  fi
  if [[ -n "$TMP_DIR" ]]; then
    rm -rf "$TMP_DIR"
  fi
  # Only a tag this run built goes; its layers stay cached for the next build.
  if [[ -z "${E2E_IMAGE:-}" ]]; then
    docker image rm "$IMAGE_TAG" >/dev/null 2>&1 || true
  fi
  exit $exit_code
}

e2e_wait_for_status() {
  local deadline=$((SECONDS + READY_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    if curl -sf -o /dev/null "http://localhost:$PORT/status"; then
      return 0
    fi
    sleep 1
  done
  return 1
}

# `fake` points this build at the fake YouTube; `real` (the released image of
# the upgrade check) uses the real one.
e2e_start_container() {
  local image="$1"
  local youtube="$2"
  local youtube_args=()
  if [[ "$youtube" == fake ]]; then
    youtube_args=(
      -e YOUTUBE_API_KEY=fake-youtube-key
      -e "YARRTUBE_YOUTUBE_API_URL=http://host.docker.internal:$YOUTUBE_STUB_PORT/youtube/v3"
      -e YTDLP_PATH=/fake-youtube/yt-dlp
      -e YARRTUBE_YTDLP_SELF_UPDATE=false
      -e DENO_DIR=/tmp/deno
      # Nothing flaky to wait out: a failing download is retried right away, so
      # the errored-download spec sees its retries run out in seconds.
      -e YARRTUBE_RETRY_BASE_DELAY_SECONDS=0
      # Downloads are instant, so steps shouldn't wait seconds for the next poll.
      -e YARRTUBE_BACKGROUND_POLL_INTERVAL_MS=250
      -v "$REPO_ROOT/e2e-tests/fake-youtube:/fake-youtube:ro"
      -v "$REPO_ROOT/e2e-tests/samples:/samples:ro"
    )
  else
    youtube_args=(
      -e YOUTUBE_API_KEY
      -e "YARRTUBE_RETRY_BASE_DELAY_SECONDS=${E2E_RETRY_BASE_DELAY_SECONDS:-10}"
    )
  fi
  echo "==> starting $image on port $PORT ($youtube YouTube)"
  docker run -d --name "$CONTAINER_NAME" \
    -p "$PORT:$PORT" \
    --add-host=host.docker.internal:host-gateway \
    -e "YARRTUBE_PLEX_URL=http://host.docker.internal:$PLEX_STUB_PORT" \
    -e YARRTUBE_PLEX_TOKEN=e2e-token \
    -e YARRTUBE_PLEX_VIDEOS_PATH=/plex/yarrtube \
    "${youtube_args[@]}" \
    -e "YARRTUBE_PORT=$PORT" \
    -e YARRTUBE_VIDEOS_PATH=/videos \
    -v "$TMP_DIR/videos:/videos" \
    -v "$TMP_DIR/config:/config" \
    "$image" >/dev/null
  CONTAINER_STARTED=true

  echo "==> waiting for http://localhost:$PORT/status to become ready (timeout ${READY_TIMEOUT_SECONDS}s)"
  if ! e2e_wait_for_status; then
    echo "error: container never became ready within ${READY_TIMEOUT_SECONDS}s" >&2
    echo "==> container logs:" >&2
    docker logs "$CONTAINER_NAME" >&2 || true
    exit 1
  fi
}

# Stops the container the way `docker stop` stops it in production, so the
# next one starts on data a graceful shutdown left behind.
e2e_stop_container() {
  docker stop "$CONTAINER_NAME" >/dev/null
  docker rm "$CONTAINER_NAME" >/dev/null
  CONTAINER_STARTED=false
}

# Runs the upgrade check's `upgrade/<phase>.spec.js` against the script's
# container (see playwright.config.js).
e2e_run_upgrade_phase() {
  BASE_URL="http://localhost:$PORT" \
    YOUTUBE_API_KEY="${YOUTUBE_API_KEY:-}" \
    E2E_UPGRADE_PHASE="$1" \
    E2E_PLAYLIST_ID="$E2E_PLAYLIST_ID" \
    E2E_PLAYLIST_NAME="$E2E_PLAYLIST_NAME" \
    E2E_VIDEOS_DIR="$TMP_DIR/videos" \
    E2E_PLEX_STUB_URL="http://localhost:$PLEX_STUB_PORT" \
    npm --prefix "$REPO_ROOT/e2e-tests" test
}

# A panic in a background task fails nothing the browser can see, so passing
# specs aren't enough on their own.
e2e_fail_on_panic() {
  if docker logs "$CONTAINER_NAME" 2>&1 | grep -q "panicked at"; then
    echo "error: the container panicked during the run (see $LOG_FILE)" >&2
    exit 1
  fi
}

cd "$REPO_ROOT"
check_dependencies
print_plan
for suite in "${ALL_SUITES[@]}"; do
  if selected "$suite"; then
    SUITE="$suite"
    case "$suite" in
      web) run_web ;;
      rust) run_rust ;;
      contract) run_contract ;;
      e2e) (run_e2e) ;;
    esac
  fi
done
