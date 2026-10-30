#!/usr/bin/env bash
# Runs the web UI from this checkout (Vite dev server, hot reload) against a
# remote yarrtube daemon's library, e.g. the one on a NAS: `/api`, `/media`
# and `/avatars` are proxied to YARRTUBE_REMOTE_URL, so there's no need to run an empty
# local server for manual UI testing. Read-only by default: every request but
# GET and HEAD is answered with 405 so clicking around can't change the
# remote library. Only frontend changes show up; backend changes need a
# remote running an image that has them (or scripts/run-local.sh).
#
# On macOS, Local Network privacy can block node (and so Vite's proxy) from
# reaching LAN addresses while Apple's own curl still can. When that happens
# the script starts a small local bridge that forwards each request through
# curl, and points Vite at it instead.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ALLOW_WRITES=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --allow-writes)
      ALLOW_WRITES=true
      shift
      ;;
    -h | --help)
      cat <<USAGE
Usage: YARRTUBE_REMOTE_URL=<url> $(basename "$0") [--allow-writes]

  YARRTUBE_REMOTE_URL  Remote daemon to browse, e.g. http://nas.local:8080
                       (from the environment or $REPO_ROOT/.env)
  --allow-writes       Forward POST/PUT/PATCH/DELETE too, so actions such as
                       mark watched, sync or delete change the remote library
USAGE
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

if [[ -z "${YARRTUBE_REMOTE_URL:-}" ]]; then
  echo "error: YARRTUBE_REMOTE_URL is not set (in the environment or $REPO_ROOT/.env)." >&2
  echo "       e.g. YARRTUBE_REMOTE_URL=http://nas.local:8080 $0" >&2
  exit 1
fi
REMOTE_URL="${YARRTUBE_REMOTE_URL%/}"

if ! curl -fsS --max-time 5 -o /dev/null "$REMOTE_URL/api/version"; then
  echo "error: $REMOTE_URL/api/version did not answer — is the daemon running and the URL right?" >&2
  exit 1
fi

if [[ ! -d "$REPO_ROOT/web/node_modules" ]]; then
  echo "==> installing web dependencies"
  (cd "$REPO_ROOT/web" && npm ci)
fi

API_URL="$REMOTE_URL"
if ! node -e '
  require("http").get(process.argv[1], { timeout: 5000 }, (res) => process.exit(res.statusCode === 200 ? 0 : 1))
    .on("error", () => process.exit(1))
    .on("timeout", () => process.exit(1));
' "$REMOTE_URL/api/version"; then
  echo "==> node can't reach $REMOTE_URL but curl can (macOS Local Network privacy?);"
  echo "    bridging the proxy through curl"
  PORT_FILE="$(mktemp)"
  # Forwards every request to the remote through a curl process, streaming
  # the response back (videos included) as it arrives. Prints the port it
  # listens on, picked by the OS, as its first line.
  node - "$REMOTE_URL" >"$PORT_FILE" <<'JS' &
const http = require('http')
const { spawn } = require('child_process')
const remote = process.argv[2]
const SKIPPED_REQUEST_HEADERS = ['host', 'connection', 'content-length', 'transfer-encoding', 'expect']
const SKIPPED_RESPONSE_HEADERS = ['connection', 'transfer-encoding', 'keep-alive']

const server = http.createServer((req, res) => {
  const head = req.method === 'HEAD'
  const hasBody = !head && req.method !== 'GET'
  const args = ['-sS', '-i', '--no-buffer', '-H', 'Expect:']
  args.push(...(head ? ['-I'] : ['-X', req.method]))
  for (const [name, value] of Object.entries(req.headers)) {
    if (!SKIPPED_REQUEST_HEADERS.includes(name)) args.push('-H', `${name}: ${value}`)
  }
  if (hasBody) args.push('--data-binary', '@-')
  args.push(remote + req.url)

  const curl = spawn('curl', args, { stdio: ['pipe', 'pipe', 'ignore'] })
  if (hasBody) req.pipe(curl.stdin)
  else curl.stdin.end()
  res.on('close', () => curl.kill())

  let pending = Buffer.alloc(0)
  let started = false
  curl.stdout.on('data', (chunk) => {
    if (started) return res.write(chunk)
    pending = Buffer.concat([pending, chunk])
    const end = pending.indexOf('\r\n\r\n')
    if (end === -1) return
    const [statusLine, ...lines] = pending.subarray(0, end).toString('latin1').split('\r\n')
    const headers = {}
    for (const line of lines) {
      const at = line.indexOf(':')
      const name = line.slice(0, at).trim().toLowerCase()
      if (SKIPPED_RESPONSE_HEADERS.includes(name)) continue
      const value = line.slice(at + 1).trim()
      headers[name] = name in headers ? [].concat(headers[name], value) : value
    }
    started = true
    res.writeHead(Number(statusLine.split(' ')[1]), headers)
    res.write(pending.subarray(end + 4))
  })
  curl.on('close', () => {
    if (!started) res.writeHead(502)
    res.end()
  })
})
server.listen(0, '127.0.0.1', () => console.log(server.address().port))
JS
  BRIDGE_PID=$!
  trap 'kill "$BRIDGE_PID" 2>/dev/null; rm -f "$PORT_FILE"' EXIT
  for _ in $(seq 50); do
    [[ -s "$PORT_FILE" ]] && break
    sleep 0.1
  done
  if [[ ! -s "$PORT_FILE" ]]; then
    echo "error: the curl bridge did not start" >&2
    exit 1
  fi
  API_URL="http://127.0.0.1:$(head -n1 "$PORT_FILE")"
fi

READ_ONLY=1
if [[ "$ALLOW_WRITES" == true ]]; then
  READ_ONLY=0
fi

echo "==> proxying /api, /media and /avatars to $REMOTE_URL"
if [[ "$API_URL" != "$REMOTE_URL" ]]; then
  echo "    via the curl bridge on $API_URL"
fi
if [[ "$READ_ONLY" == 1 ]]; then
  echo "    read-only: writes are answered with 405 (pass --allow-writes to forward them)"
else
  echo "    writes ENABLED: actions change the remote library"
fi

cd "$REPO_ROOT/web"
YARRTUBE_API_URL="$API_URL" YARRTUBE_API_READ_ONLY="$READ_ONLY" npm run dev
