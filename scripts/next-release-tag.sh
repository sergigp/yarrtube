#!/usr/bin/env bash
# Prints the tag the next release should get, or nothing when HEAD is
# already tagged. Run by CI after every merge to main.
#
# The version in Cargo.toml wins when it's ahead of the latest tag, so a
# minor/major bump is cut by raising it in the merged PR; otherwise the
# latest tag's patch number is bumped.
set -euo pipefail

if git tag --points-at HEAD | grep -qE '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
  exit 0
fi

latest=$(git tag --list 'v*.*.*' --sort=-v:refname | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -n1 || true)
latest=${latest#v}
cargo=$(sed -nE 's/^version = "([0-9]+\.[0-9]+\.[0-9]+)"$/\1/p' Cargo.toml | head -n1)

if [ -z "$latest" ]; then
  echo "v${cargo}"
  exit 0
fi

highest=$(printf '%s\n%s\n' "$latest" "$cargo" | sort -V | tail -n1)
if [ "$cargo" != "$latest" ] && [ "$highest" = "$cargo" ]; then
  echo "v${cargo}"
else
  IFS=. read -r major minor patch <<< "$latest"
  echo "v${major}.${minor}.$((patch + 1))"
fi
