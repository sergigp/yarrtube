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
# end-to-end tests run at once. In CI (`--ci`, or `CI=true`,
# which GitHub Actions sets) the contract tests are left out: YouTube
# bot-checks CI runners. It only runs checks: dependencies, the Playwright
# browsers and the web UI build must already be in place.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ALL_SUITES=(web rust contract e2e)

CI_MODE=false
if [[ "${CI:-}" == "true" ]]; then
  CI_MODE=true
fi
ONLY=()
WORKERS=1
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
      WORKERS="${2:-}"
      shift 2
      ;;
    -h | --help)
      sed -n '2,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

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

selected() {
  local suite="$1"
  if [[ ${#ONLY[@]} -gt 0 ]]; then
    [[ " ${ONLY[*]} " == *" $suite "* ]]
  else
    [[ "$suite" != contract || "$CI_MODE" == false ]]
  fi
}

require_dir() {
  local dir="$1" how="$2"
  if [[ ! -d "$REPO_ROOT/$dir" ]]; then
    echo "error: $dir is missing; run '$how' first" >&2
    exit 1
  fi
}

run_web() {
  require_dir web/node_modules "npm --prefix web ci"
  step "web UI: typecheck, lint, test"
  npm --prefix "$REPO_ROOT/web" run check
}

run_rust() {
  # The Rust binary embeds the built web UI.
  require_dir web/dist "npm --prefix web ci && npm --prefix web run build"
  step "Rust: formatting"
  cargo fmt --all -- --check
  step "Rust: clippy"
  cargo clippy --all-targets --all-features --locked -- -D warnings
  step "Rust: tests"
  cargo test --locked
}

# The adapters' `#[ignore]`d tests against the real yt-dlp and YouTube Data
# API (modules named `youtube_contract_tests`). Run them after upgrading
# yt-dlp or when downloads start failing, to see whether YouTube's side
# changed.
run_contract() {
  require_dir web/dist "npm --prefix web ci && npm --prefix web run build"
  : "${YOUTUBE_API_KEY:?YOUTUBE_API_KEY must be set (in the environment or $REPO_ROOT/.env)}"
  local ytdlp_path="${YTDLP_PATH:-$(command -v yt-dlp || true)}"
  if [[ -z "$ytdlp_path" ]]; then
    echo "error: yt-dlp not found on PATH (or set YTDLP_PATH)" >&2
    exit 1
  fi
  if ! command -v ffprobe >/dev/null; then
    echo "error: ffprobe not found on PATH (it comes with ffmpeg, which yt-dlp also needs)" >&2
    exit 1
  fi
  step "YouTube contract tests (yt-dlp $("$ytdlp_path" --version) at $ytdlp_path)"
  YOUTUBE_API_KEY="$YOUTUBE_API_KEY" YTDLP_PATH="$ytdlp_path" \
    cargo test --locked -- --ignored youtube_contract
}

# Builds the release Docker image (or takes `E2E_IMAGE`), runs it with fresh
# temp database/videos directories against a fake YouTube, and drives it with
# the Playwright suite. Before that, the upgrade check: the released image
# builds up a library, then this build starts on the same database and videos
# and must carry it on as it was. Everything a run creates carries its own id
# and ports, so runs from different worktrees can go at the same time. Run in
# a subshell, so its cleanup trap is its own.
run_e2e() {
  require_dir e2e-tests/node_modules \
    "npm --prefix e2e-tests ci && npx --prefix e2e-tests playwright install --with-deps chromium webkit"
  step "end-to-end suite"

  local run_id
  run_id="$(date +%s)-$$"
  IMAGE_TAG="${E2E_IMAGE:-yarrtube:e2e-$run_id}"
  # The image the upgrade check starts from: the latest release, so every
  # change is checked against the data the version users run today leaves
  # behind. `none` skips the check.
  UPGRADE_FROM_IMAGE="${E2E_UPGRADE_FROM_IMAGE:-ghcr.io/sergigp/yarrtube:latest}"
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

  # The suite runs against a fake YouTube (see e2e-tests/README.md). Only
  # the upgrade check needs the real one, since the released image it starts
  # can't be pointed at the fake: it runs only with a real YOUTUBE_API_KEY.
  if [[ "$UPGRADE_FROM_IMAGE" != "none" && -z "${YOUTUBE_API_KEY:-}" ]]; then
    echo "==> no YOUTUBE_API_KEY: skipping the upgrade check, which needs the real YouTube"
    UPGRADE_FROM_IMAGE="none"
  fi
  # The upgrade check's fixture: the maintainer-owned "test" playlist on the
  # real YouTube.
  E2E_PLAYLIST_ID="${E2E_PLAYLIST_ID:-PLXWRoRTUXjks}"
  E2E_PLAYLIST_NAME="${E2E_PLAYLIST_NAME:-test}"

  TMP_DIR=""
  CONTAINER_STARTED=false
  PLEX_STUB_PID=""
  YOUTUBE_STUB_PID=""
  trap e2e_cleanup EXIT

  if [[ -n "${E2E_IMAGE:-}" ]]; then
    echo "==> testing $IMAGE_TAG"
  else
    echo "==> building $IMAGE_TAG"
    docker build -t "$IMAGE_TAG" "$REPO_ROOT"
  fi

  # The fake YouTube Data API on the host, shared by every daemon (it only
  # reads); the stand-in yt-dlp is mounted into each container (see
  # e2e-tests/fake-youtube/).
  echo "==> starting YouTube stub on port $YOUTUBE_STUB_PORT"
  E2E_YOUTUBE_STUB_PORT="$YOUTUBE_STUB_PORT" node "$REPO_ROOT/e2e-tests/fake-youtube/youtube-stub.js" &
  YOUTUBE_STUB_PID=$!

  if [[ "$UPGRADE_FROM_IMAGE" != "none" ]]; then
    TMP_DIR="$(mktemp -d /tmp/yarrtube-e2e.XXXXXX)"
    mkdir -p "$TMP_DIR/videos" "$TMP_DIR/config"
    # A stand-in Plex server on the host, so the check can see yarrtube asks
    # Plex to scan what it downloads (see e2e-tests/plex-stub.js).
    echo "==> starting Plex stub on port $PLEX_STUB_PORT"
    E2E_PLEX_STUB_PORT="$PLEX_STUB_PORT" node "$REPO_ROOT/e2e-tests/plex-stub.js" &
    PLEX_STUB_PID=$!

    echo "==> upgrade check: seeding a library with $UPGRADE_FROM_IMAGE"
    docker pull "$UPGRADE_FROM_IMAGE"
    e2e_start_container "$UPGRADE_FROM_IMAGE" real
    e2e_run_upgrade_phase seed
    e2e_stop_container

    echo "==> upgrade check: verifying the seeded library on $IMAGE_TAG"
    e2e_start_container "$IMAGE_TAG" fake
    e2e_run_upgrade_phase verify
    e2e_fail_on_panic
    e2e_stop_container
  fi

  # Each worker starts its own daemon from the image (see
  # e2e-tests/helpers/daemon.js).
  echo "==> running Playwright suite on $WORKERS worker(s)"
  E2E_IMAGE="$IMAGE_TAG" \
    E2E_RUN_ID="$run_id" \
    E2E_YOUTUBE_STUB_PORT="$YOUTUBE_STUB_PORT" \
    E2E_WORKERS="$WORKERS" \
    E2E_CHECK_ROUTE_COVERAGE=1 \
    npm --prefix "$REPO_ROOT/e2e-tests" test
}

e2e_cleanup() {
  local exit_code=$?
  if [[ "$CONTAINER_STARTED" == true ]]; then
    if [[ $exit_code -ne 0 ]]; then
      echo "==> run failed (exit $exit_code); saving container logs to $LOG_FILE"
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
for suite in "${ALL_SUITES[@]}"; do
  if selected "$suite"; then
    case "$suite" in
      web) run_web ;;
      rust) run_rust ;;
      contract) run_contract ;;
      e2e) (run_e2e) ;;
    esac
  else
    step "skipped: $suite"
  fi
done

step "all selected suites passed"
