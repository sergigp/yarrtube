#!/usr/bin/env bash
# Runs `cargo run -- serve` for manual testing. The SQLite database lives in
# the repo root (already gitignored) and, by default, so do downloaded
# videos — both need to persist across restarts, since the database keeps
# referencing videos by path: wiping just the videos directory on every run
# (the old behavior) left the database pointing at files that no longer
# existed, breaking playback of anything downloaded before the last restart.
# Pass --fresh-videos for the old ephemeral-/tmp-directory behavior.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${YARRTUBE_PORT:-8080}"
SKIP_WEB_BUILD=false
FRESH_VIDEOS=false

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
    --fresh-videos)
      FRESH_VIDEOS=true
      shift
      ;;
    -h | --help)
      cat <<USAGE
Usage: $(basename "$0") [--port <port>] [--skip-web-build] [--fresh-videos]

  --port <port>        HTTP port to listen on (default: 8080)
  --skip-web-build      Reuse the existing web/dist instead of rebuilding it
  --fresh-videos         Use a fresh, throwaway /tmp directory for downloaded
                         videos instead of the persistent one under the repo
                         (breaks playback of anything downloaded in a
                         previous run, since the database keeps its old
                         file paths)
USAGE
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

YTDLP_ON_PATH="$(command -v yt-dlp || true)"
if [[ -z "$YTDLP_ON_PATH" ]]; then
  echo "error: yt-dlp not found on PATH — install it first (see README.md)" >&2
  exit 1
fi

if [[ -f "$REPO_ROOT/.env" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
  set +a
fi

# yarrtube defaults YTDLP_PATH to /app/bin/yt-dlp (the Docker image's managed
# location); locally that path doesn't exist, so point it at the yt-dlp
# already found on PATH above, unless the environment/.env already set one.
YTDLP_PATH="${YTDLP_PATH:-$YTDLP_ON_PATH}"

if [[ -z "${YOUTUBE_API_KEY:-}" ]]; then
  echo "warning: YOUTUBE_API_KEY is not set (in the environment or $REPO_ROOT/.env)." >&2
  echo "         Tracking a YouTube-linked playlist will fail; custom playlists still work." >&2
fi

if [[ "$SKIP_WEB_BUILD" == false ]]; then
  echo "==> building web UI"
  (cd "$REPO_ROOT/web" && npm ci && npm run build)
fi

DB_PATH="$REPO_ROOT/yarrtube.sqlite3"
if [[ "$FRESH_VIDEOS" == true ]]; then
  VIDEOS_PATH="$(mktemp -d /tmp/yarrtube-local-videos.XXXXXX)"
else
  VIDEOS_PATH="${YARRTUBE_VIDEOS_PATH:-$REPO_ROOT/videos}"
  mkdir -p "$VIDEOS_PATH"
fi
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
echo "    YTDLP_PATH=$YTDLP_PATH"
echo "    RUST_LOG=$RUST_LOG_VALUE"
echo "==> starting yarrtube serve on http://localhost:$PORT (Ctrl+C to stop)"

cd "$REPO_ROOT"
exec env \
  YOUTUBE_API_KEY="${YOUTUBE_API_KEY:-}" \
  YARRTUBE_PORT="$PORT" \
  YARRTUBE_DB_PATH="$DB_PATH" \
  YARRTUBE_VIDEOS_PATH="$VIDEOS_PATH" \
  YTDLP_PATH="$YTDLP_PATH" \
  RUST_LOG="$RUST_LOG_VALUE" \
  cargo run -- serve
