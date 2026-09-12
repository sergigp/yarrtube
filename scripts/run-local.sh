#!/usr/bin/env bash
# Builds and runs yarrtube locally for manual testing. The SQLite database
# and downloaded videos are written under a fresh directory in /tmp so test
# data doesn't linger in the repo or a real media library.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${YARRTUBE_PORT:-8080}"
SKIP_BUILD=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      PORT="$2"
      shift 2
      ;;
    --skip-build)
      SKIP_BUILD=true
      shift
      ;;
    -h | --help)
      cat <<USAGE
Usage: $(basename "$0") [--port <port>] [--skip-build]

  --port <port>   HTTP port to listen on (default: 8080)
  --skip-build    Reuse the existing web/dist and target/release build
                  instead of rebuilding both
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

if [[ "$SKIP_BUILD" == false ]]; then
  echo "==> building web UI"
  (cd "$REPO_ROOT/web" && npm ci && npm run build)

  echo "==> building yarrtube (release)"
  (cd "$REPO_ROOT" && cargo build --release --locked)
fi

WORKDIR="$(mktemp -d /tmp/yarrtube-local.XXXXXX)"
mkdir -p "$WORKDIR/videos"

echo "==> working directory: $WORKDIR"
echo "==> starting yarrtube serve on http://localhost:$PORT"
echo "    Ctrl+C to stop. Data stays under /tmp until macOS reclaims it."

exec env \
  YOUTUBE_API_KEY="${YOUTUBE_API_KEY:-}" \
  YARRTUBE_PORT="$PORT" \
  YARRTUBE_DB_PATH="$WORKDIR/yarrtube.sqlite3" \
  YARRTUBE_VIDEOS_PATH="$WORKDIR/videos" \
  RUST_LOG="${RUST_LOG:-info}" \
  "$REPO_ROOT/target/release/yarrtube" serve
