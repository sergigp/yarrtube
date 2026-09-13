#!/bin/sh
set -e

PUID="${PUID:-0}"
PGID="${PGID:-0}"

# Default (no PUID/PGID set): keep the historical behavior and stay root.
if [ "$PUID" = "0" ] && [ "$PGID" = "0" ]; then
    exec "$@"
fi

if ! getent group "$PGID" >/dev/null 2>&1; then
    addgroup --gid "$PGID" yarrtube
fi
group_name="$(getent group "$PGID" | cut -d: -f1)"

if ! getent passwd "$PUID" >/dev/null 2>&1; then
    adduser --uid "$PUID" --ingroup "$group_name" --disabled-password --gecos "" --no-create-home yarrtube
fi
user_name="$(getent passwd "$PUID" | cut -d: -f1)"

# /app holds the SQLite database the daemon writes to; it's small, so a full
# chown on every start is cheap. /videos can be huge (a whole media library),
# so only the mount point itself is chowned — new files/directories the
# daemon creates under it inherit the right owner from then on, without
# recursively touching pre-existing content on every restart.
chown -R "$PUID:$PGID" /app
mkdir -p /videos
chown "$PUID:$PGID" /videos

exec gosu "$user_name:$group_name" "$@"
