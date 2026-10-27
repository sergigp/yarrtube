import { execFile } from 'node:child_process'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { startPlexStub } from '../plex-stub.js'

const E2E_DIR = join(dirname(fileURLToPath(import.meta.url)), '..')
const READY_TIMEOUT_MS = 120_000
const PLEX_TOKEN = 'e2e-token'
const run = promisify(execFile)

/**
 * Starts this worker's own yarrtube daemon: the image under test (`image`) in
 * a container of its own, with a fresh database and videos directory, on a
 * free port, pointed at the shared fake YouTube (`youtubeStubPort`) and at a
 * Plex stub of its own. During the upgrade check's phases nothing is started:
 * those run against the container `scripts/run-tests.sh` manages.
 *
 * `stop()` saves the container's log to `test-results/`, removes the
 * container, its directories and the Plex stub, and fails if the daemon
 * logged a panic, which nothing the browser sees would reveal.
 */
export async function startDaemon({ runId, workerIndex, image, youtubeStubPort }) {
  if (process.env.E2E_UPGRADE_PHASE) {
    return {
      url: process.env.BASE_URL,
      videosDir: process.env.E2E_VIDEOS_DIR,
      plexStubUrl: process.env.E2E_PLEX_STUB_URL,
      stop: async () => {},
    }
  }
  const [port, plexPort] = await freePorts(2)
  const dir = mkdtempSync(join(tmpdir(), 'yarrtube-e2e-'))
  const videosDir = join(dir, 'videos')
  mkdirSync(videosDir)
  mkdirSync(join(dir, 'config'))
  const plex = await startPlexStub(plexPort)
  const name = `yarrtube-e2e-${runId}-w${workerIndex}`
  await run('docker', [
    'run', '-d', '--name', name,
    '-p', `${port}:${port}`,
    '--add-host=host.docker.internal:host-gateway',
    '-e', `YARRTUBE_PORT=${port}`,
    '-e', 'YARRTUBE_VIDEOS_PATH=/videos',
    '-e', `YARRTUBE_PLEX_URL=http://host.docker.internal:${plexPort}`,
    '-e', `YARRTUBE_PLEX_TOKEN=${PLEX_TOKEN}`,
    '-e', 'YARRTUBE_PLEX_VIDEOS_PATH=/plex/yarrtube',
    ...fakeYoutubeArgs(youtubeStubPort),
    '-v', `${videosDir}:/videos`,
    '-v', `${join(dir, 'config')}:/config`,
    image,
  ])
  const url = `http://localhost:${port}`
  const stop = () => stopDaemon({ name, dir, plex, runId, workerIndex })
  try {
    await waitForStatus(url)
  } catch (error) {
    await stop().catch(() => {})
    throw error
  }
  return { url, videosDir, plexStubUrl: `http://localhost:${plexPort}`, stop }
}

// The daemon against the fake YouTube (see `fake-youtube/`): the stand-in
// yt-dlp and the samples are mounted read-only, the self-update that would
// replace the stand-in is off, failed downloads are retried right away (the
// errored-download spec runs its retries out in seconds) and background work
// is picked up every 250 ms, since downloads are instant.
function fakeYoutubeArgs(youtubeStubPort) {
  return [
    '-e', 'YOUTUBE_API_KEY=fake-youtube-key',
    '-e', `YARRTUBE_YOUTUBE_API_URL=http://host.docker.internal:${youtubeStubPort}/youtube/v3`,
    '-e', 'YTDLP_PATH=/fake-youtube/yt-dlp',
    '-e', 'YARRTUBE_YTDLP_SELF_UPDATE=false',
    '-e', 'DENO_DIR=/tmp/deno',
    '-e', 'YARRTUBE_RETRY_BASE_DELAY_SECONDS=0',
    '-e', 'YARRTUBE_BACKGROUND_POLL_INTERVAL_MS=250',
    '-v', `${join(E2E_DIR, 'fake-youtube')}:/fake-youtube:ro`,
    '-v', `${join(E2E_DIR, 'samples')}:/samples:ro`,
  ]
}

async function stopDaemon({ name, dir, plex, runId, workerIndex }) {
  const { stdout, stderr } = await run('docker', ['logs', name], { maxBuffer: 64 * 1024 * 1024 })
  const log = stdout + stderr
  const logFile = join(E2E_DIR, 'test-results', `container-${runId}-w${workerIndex}.log`)
  mkdirSync(dirname(logFile), { recursive: true })
  writeFileSync(logFile, log)
  // The daemon writes its files as root, which on Linux the host can't
  // remove: the container empties its own mounts first.
  await run('docker', ['exec', name, 'find', '/videos', '/config', '-mindepth', '1', '-delete'])
  await run('docker', ['rm', '-f', name])
  await plex.close()
  rmSync(dir, { recursive: true, force: true, maxRetries: 2 })
  if (log.includes('panicked at')) {
    throw new Error(`the daemon ${name} panicked (see ${logFile})`)
  }
}

async function waitForStatus(url) {
  const deadline = Date.now() + READY_TIMEOUT_MS
  while (Date.now() < deadline) {
    try {
      if ((await fetch(`${url}/status`)).ok) {
        return
      }
    } catch {
      // Not listening yet.
    }
    await new Promise((resolve) => setTimeout(resolve, 250))
  }
  throw new Error(`the daemon at ${url} wasn't ready within ${READY_TIMEOUT_MS / 1000} s`)
}

// Asked for together, so they can't collide with each other.
async function freePorts(count) {
  const servers = await Promise.all(
    Array.from({ length: count }, () => new Promise((resolve) => {
      const server = createServer().listen(0, '127.0.0.1', () => resolve(server))
    })),
  )
  const ports = servers.map((server) => server.address().port)
  await Promise.all(servers.map((server) => new Promise((resolve) => server.close(resolve))))
  return ports
}
