#!/usr/bin/env bash
# Builds the release Docker image, runs it with fresh temp DB/videos
# directories, waits for it to become ready, then runs the smoke-tests
# Playwright suite against it. Used identically by developers locally and by
# CI (.github/workflows/smoke-tests.yml) so both take the same path.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE_TAG="yarrtube:smoke"
CONTAINER_NAME="yarrtube-smoke"
PORT="${YARRTUBE_PORT:-8080}"
READY_TIMEOUT_SECONDS="${SMOKE_READY_TIMEOUT_SECONDS:-120}"
LOG_FILE="$REPO_ROOT/smoke-tests/container.log"

if [[ -f "$REPO_ROOT/.env" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
  set +a
fi

: "${YOUTUBE_API_KEY:?YOUTUBE_API_KEY must be set (in the environment or $REPO_ROOT/.env)}"
: "${SMOKE_PLAYLIST_ID:?SMOKE_PLAYLIST_ID must be set — the id of the maintainer-owned \"yarrtube-smoke-tests\" YouTube playlist (see smoke-tests/README.md)}"
SMOKE_PLAYLIST_NAME="${SMOKE_PLAYLIST_NAME:-yarrtube smoke tests}"
SMOKE_CHANNEL_HANDLE="${SMOKE_CHANNEL_HANDLE:-@BlenderOfficial}"
SMOKE_CHANNEL_VIDEO_LIMIT="${SMOKE_CHANNEL_VIDEO_LIMIT:-1}"

TMP_DIR=""
CONTAINER_STARTED=false

cleanup() {
  local exit_code=$?
  if [[ "$CONTAINER_STARTED" == true ]]; then
    if [[ $exit_code -ne 0 ]]; then
      echo "==> run failed (exit $exit_code); saving container logs to $LOG_FILE"
      docker logs "$CONTAINER_NAME" >"$LOG_FILE" 2>&1 || true
    fi
    docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
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

TMP_DIR="$(mktemp -d /tmp/yarrtube-smoke.XXXXXX)"
mkdir -p "$TMP_DIR/videos" "$TMP_DIR/data"

echo "==> starting container on port $PORT"
docker run -d --name "$CONTAINER_NAME" \
  -p "$PORT:$PORT" \
  -e YOUTUBE_API_KEY \
  -e "YARRTUBE_PORT=$PORT" \
  -e YARRTUBE_DB_PATH=/data/yarrtube.sqlite3 \
  -e YARRTUBE_VIDEOS_PATH=/videos \
  -v "$TMP_DIR/videos:/videos" \
  -v "$TMP_DIR/data:/data" \
  "$IMAGE_TAG" >/dev/null
CONTAINER_STARTED=true

echo "==> waiting for http://localhost:$PORT/status to become ready (timeout ${READY_TIMEOUT_SECONDS}s)"
if ! wait_for_status; then
  echo "error: container never became ready within ${READY_TIMEOUT_SECONDS}s" >&2
  echo "==> container logs:" >&2
  docker logs "$CONTAINER_NAME" >&2 || true
  exit 1
fi

echo "==> installing smoke-tests dependencies"
npm --prefix "$REPO_ROOT/smoke-tests" ci

echo "==> running Playwright suite"
BASE_URL="http://localhost:$PORT" \
  YOUTUBE_API_KEY="$YOUTUBE_API_KEY" \
  SMOKE_PLAYLIST_ID="$SMOKE_PLAYLIST_ID" \
  SMOKE_PLAYLIST_NAME="$SMOKE_PLAYLIST_NAME" \
  SMOKE_CHANNEL_HANDLE="$SMOKE_CHANNEL_HANDLE" \
  SMOKE_CHANNEL_VIDEO_LIMIT="$SMOKE_CHANNEL_VIDEO_LIMIT" \
  npm --prefix "$REPO_ROOT/smoke-tests" test
