#!/usr/bin/env bash
# Builds the release Docker image, runs it with fresh temp DB/videos
# directories, waits for it to become ready, then runs the smoke-tests
# Playwright suite against it. Used identically by developers locally and by
# CI (.github/workflows/smoke-tests.yml) so both take the same path.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE_TAG="yarrtube:smoke"
# The image the upgrade check starts from: the latest release, so every
# change is checked against the data the version users run today leaves
# behind. `none` skips the check.
UPGRADE_FROM_IMAGE="${SMOKE_UPGRADE_FROM_IMAGE:-ghcr.io/sergigp/yarrtube:latest}"
CONTAINER_NAME="yarrtube-smoke"
PORT="${YARRTUBE_PORT:-8080}"
READY_TIMEOUT_SECONDS="${SMOKE_READY_TIMEOUT_SECONDS:-120}"
PLEX_STUB_PORT="${SMOKE_PLEX_STUB_PORT:-18324}"
LOG_FILE="$REPO_ROOT/smoke-tests/container.log"

if [[ -f "$REPO_ROOT/.env" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
  set +a
fi

: "${YOUTUBE_API_KEY:?YOUTUBE_API_KEY must be set (in the environment or $REPO_ROOT/.env)}"
# The maintainer-owned "yarrtube-smoke-tests" playlist (see smoke-tests/README.md).
SMOKE_PLAYLIST_ID="${SMOKE_PLAYLIST_ID:-PLXWRoRTUXjks}"
SMOKE_PLAYLIST_NAME="${SMOKE_PLAYLIST_NAME:-test}"
SMOKE_CHANNEL_HANDLE="${SMOKE_CHANNEL_HANDLE:-@BlenderOfficial}"
SMOKE_CHANNEL_VIDEO_LIMIT="${SMOKE_CHANNEL_VIDEO_LIMIT:-1}"

TMP_DIR=""
CONTAINER_STARTED=false
PLEX_STUB_PID=""

cleanup() {
  local exit_code=$?
  if [[ "$CONTAINER_STARTED" == true ]]; then
    if [[ $exit_code -ne 0 ]]; then
      echo "==> run failed (exit $exit_code); saving container logs to $LOG_FILE"
      docker logs "$CONTAINER_NAME" >"$LOG_FILE" 2>&1 || true
    fi
    docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
  fi
  if [[ -n "$PLEX_STUB_PID" ]]; then
    kill "$PLEX_STUB_PID" 2>/dev/null || true
  fi
  if [[ -n "$TMP_DIR" ]]; then
    rm -rf "$TMP_DIR"
  fi
  exit $exit_code
}
trap cleanup EXIT

wait_for_status() {
  local deadline=$((SECONDS + READY_TIMEOUT_SECONDS))
  while (( SECONDS < deadline )); do
    if curl -sf -o /dev/null "http://localhost:$PORT/status"; then
      return 0
    fi
    sleep 1
  done
  return 1
}

echo "==> building $IMAGE_TAG"
docker build -t "$IMAGE_TAG" "$REPO_ROOT"

# In case a previous run was killed before its cleanup trap could fire.
docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true

TMP_DIR="$(mktemp -d /tmp/yarrtube-smoke.XXXXXX)"
mkdir -p "$TMP_DIR/videos" "$TMP_DIR/config"

# A stand-in Plex server on the host, so specs can check yarrtube asks Plex
# to scan what it downloads (see smoke-tests/plex-stub.js).
echo "==> starting Plex stub on port $PLEX_STUB_PORT"
SMOKE_PLEX_STUB_PORT="$PLEX_STUB_PORT" node "$REPO_ROOT/smoke-tests/plex-stub.js" &
PLEX_STUB_PID=$!

start_container() {
  local image="$1"
  echo "==> starting $image on port $PORT"
  docker run -d --name "$CONTAINER_NAME" \
    -p "$PORT:$PORT" \
    --add-host=host.docker.internal:host-gateway \
    -e "YARRTUBE_PLEX_URL=http://host.docker.internal:$PLEX_STUB_PORT" \
    -e YARRTUBE_PLEX_TOKEN=smoke-token \
    -e YARRTUBE_PLEX_VIDEOS_PATH=/plex/yarrtube \
    -e YOUTUBE_API_KEY \
    -e "YARRTUBE_PORT=$PORT" \
    -e YARRTUBE_VIDEOS_PATH=/videos \
    -e "YARRTUBE_RETRY_BASE_DELAY_SECONDS=${SMOKE_RETRY_BASE_DELAY_SECONDS:-10}" \
    -v "$TMP_DIR/videos:/videos" \
    -v "$TMP_DIR/config:/config" \
    "$image" >/dev/null
  CONTAINER_STARTED=true

  echo "==> waiting for http://localhost:$PORT/status to become ready (timeout ${READY_TIMEOUT_SECONDS}s)"
  if ! wait_for_status; then
    echo "error: container never became ready within ${READY_TIMEOUT_SECONDS}s" >&2
    echo "==> container logs:" >&2
    docker logs "$CONTAINER_NAME" >&2 || true
    exit 1
  fi
}

# Stops the container the way `docker stop` stops it in production, so the
# next one starts on data a graceful shutdown left behind.
stop_container() {
  docker stop "$CONTAINER_NAME" >/dev/null
  docker rm "$CONTAINER_NAME" >/dev/null
  CONTAINER_STARTED=false
}

# `SMOKE_UPGRADE_PHASE` selects the `upgrade/<phase>.spec.js` project instead
# of the main suite (see playwright.config.js).
run_playwright() {
  BASE_URL="http://localhost:$PORT" \
    YOUTUBE_API_KEY="$YOUTUBE_API_KEY" \
    SMOKE_PLAYLIST_ID="$SMOKE_PLAYLIST_ID" \
    SMOKE_PLAYLIST_NAME="$SMOKE_PLAYLIST_NAME" \
    SMOKE_CHANNEL_HANDLE="$SMOKE_CHANNEL_HANDLE" \
    SMOKE_CHANNEL_VIDEO_LIMIT="$SMOKE_CHANNEL_VIDEO_LIMIT" \
    SMOKE_VIDEOS_DIR="$TMP_DIR/videos" \
    SMOKE_PLEX_STUB_URL="http://localhost:$PLEX_STUB_PORT" \
    "$@" \
    npm --prefix "$REPO_ROOT/smoke-tests" test
}

echo "==> installing smoke-tests dependencies"
npm --prefix "$REPO_ROOT/smoke-tests" ci
npx --prefix "$REPO_ROOT/smoke-tests" playwright install --with-deps chromium

# Upgrade check: the released image builds up a library, then this build
# starts on the same database and videos and must carry it on as it was.
if [[ "$UPGRADE_FROM_IMAGE" != "none" ]]; then
  echo "==> upgrade check: seeding a library with $UPGRADE_FROM_IMAGE"
  docker pull "$UPGRADE_FROM_IMAGE"
  start_container "$UPGRADE_FROM_IMAGE"
  run_playwright env SMOKE_UPGRADE_PHASE=seed
  stop_container
fi

start_container "$IMAGE_TAG"

if [[ "$UPGRADE_FROM_IMAGE" != "none" ]]; then
  echo "==> upgrade check: verifying the seeded library on $IMAGE_TAG"
  run_playwright env SMOKE_UPGRADE_PHASE=verify
fi

echo "==> running Playwright suite"
run_playwright env SMOKE_CHECK_ROUTE_COVERAGE=1

# A panic in a background task fails nothing the browser can see, so the
# suite passing isn't enough on its own.
if docker logs "$CONTAINER_NAME" 2>&1 | grep -q "panicked at"; then
  echo "error: the container panicked during the run (see $LOG_FILE)" >&2
  exit 1
fi
