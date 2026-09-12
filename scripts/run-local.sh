#!/usr/bin/env bash
# Runs `cargo run -- serve` for manual testing. The SQLite database lives in
# the repo root (already gitignored); downloaded videos go to a fresh /tmp
# directory so they don't linger on disk.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${YARRTUBE_PORT:-8080}"
SKIP_WEB_BUILD=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      PORT="$2"
      shift 2
      ;;
    --skip-web-build)
      SKIP_WEB_BUILD=true
      shift
      ;;
    -h | --help)
      cat <<USAGE
Usage: $(basename "$0") [--port <port>] [--skip-web-build]

  --port <port>        HTTP port to listen on (default: 8080)
  --skip-web-build      Reuse the existing web/dist instead of rebuilding it
USAGE
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

command -v yt-dlp >/dev/null || {
  echo "error: yt-dlp not found on PATH — install it first (see README.md)" >&2
  exit 1
}

if [[ -f "$REPO_ROOT/.env" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
  set +a
fi

if [[ -z "${YOUTUBE_API_KEY:-}" ]]; then
  echo "warning: YOUTUBE_API_KEY is not set (in the environment or $REPO_ROOT/.env)." >&2
  echo "         Tracking a YouTube-linked playlist will fail; custom playlists still work." >&2
fi

if [[ "$SKIP_WEB_BUILD" == false ]]; then
  echo "==> building web UI"
  (cd "$REPO_ROOT/web" && npm ci && npm run build)
fi

DB_PATH="$REPO_ROOT/yarrtube.sqlite3"
VIDEOS_PATH="$(mktemp -d /tmp/yarrtube-local-videos.XXXXXX)"
RUST_LOG_VALUE="${RUST_LOG:-info}"

echo "==> environment injected into yarrtube:"
if [[ -n "${YOUTUBE_API_KEY:-}" ]]; then
  echo "    YOUTUBE_API_KEY=<set>"
else
  echo "    YOUTUBE_API_KEY=<not set>"
fi
echo "    YARRTUBE_PORT=$PORT"
echo "    YARRTUBE_DB_PATH=$DB_PATH"
echo "    YARRTUBE_VIDEOS_PATH=$VIDEOS_PATH"
echo "    RUST_LOG=$RUST_LOG_VALUE"
echo "==> starting yarrtube serve on http://localhost:$PORT (Ctrl+C to stop)"

cd "$REPO_ROOT"
exec env \
  YOUTUBE_API_KEY="${YOUTUBE_API_KEY:-}" \
  YARRTUBE_PORT="$PORT" \
  YARRTUBE_DB_PATH="$DB_PATH" \
  YARRTUBE_VIDEOS_PATH="$VIDEOS_PATH" \
  RUST_LOG="$RUST_LOG_VALUE" \
  cargo run -- serve
