#!/usr/bin/env bash
# Runs the YouTube contract tests: the adapters' `#[ignore]`d tests against
# the real `yt-dlp` and YouTube Data API (modules named
# `youtube_contract_tests`). CI and the default `cargo test` skip them, since
# they need the network, a YouTube API key, and an IP YouTube doesn't
# bot-check. Run them after upgrading `yt-dlp` or when downloads start
# failing, to see whether YouTube's side changed.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "$REPO_ROOT/.env" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
  set +a
fi

: "${YOUTUBE_API_KEY:?YOUTUBE_API_KEY must be set (in the environment or $REPO_ROOT/.env)}"
YTDLP_PATH="${YTDLP_PATH:-$(command -v yt-dlp || true)}"
if [[ -z "$YTDLP_PATH" ]]; then
  echo "error: yt-dlp not found on PATH (or set YTDLP_PATH)" >&2
  exit 1
fi
if ! command -v ffprobe >/dev/null; then
  echo "error: ffprobe not found on PATH (it comes with ffmpeg, which yt-dlp also needs)" >&2
  exit 1
fi
export YOUTUBE_API_KEY YTDLP_PATH

echo "==> yt-dlp $("$YTDLP_PATH" --version) at $YTDLP_PATH"
cd "$REPO_ROOT"
cargo test --locked -- --ignored youtube_contract
