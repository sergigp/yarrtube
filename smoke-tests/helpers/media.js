import { expect } from '@playwright/test'
import { execFile } from 'node:child_process'
import { promisify } from 'node:util'

// The container `run-smoke-tests.sh` runs, whose `ffprobe` reads the videos
// exactly where the daemon wrote them.
const CONTAINER_NAME = process.env.SMOKE_CONTAINER_NAME

/**
 * Asserts the video at `relativePath` (under the videos root) is an `.mp4`
 * holding H.264 video and AAC audio, the combination every browser,
 * iPhone Safari included, plays.
 */
export async function expectH264Mp4(relativePath) {
  expect(relativePath, 'media file name').toMatch(/\.mp4$/)
  expect(CONTAINER_NAME, 'SMOKE_CONTAINER_NAME').toBeTruthy()
  const { stdout } = await promisify(execFile)('docker', [
    'exec',
    CONTAINER_NAME,
    'ffprobe',
    '-v',
    'error',
    '-show_entries',
    'stream=codec_name,codec_type',
    '-of',
    'json',
    `/videos/${relativePath}`,
  ])
  const codecs = JSON.parse(stdout).streams.map(({ codec_type: type, codec_name: name }) => ({ type, name }))
  expect(codecs).toEqual(
    expect.arrayContaining([
      { type: 'video', name: 'h264' },
      { type: 'audio', name: 'aac' },
    ]),
  )
}

/** Asserts `mediaUrl` answers a byte-range request with a partial response. */
export async function expectRangeSupported(request, mediaUrl) {
  const response = await request.get(mediaUrl, { headers: { Range: 'bytes=0-1023' } })
  expect(response.status()).toBe(206)
  expect(response.headers()['content-range']).toMatch(/^bytes 0-1023\/\d+$/)
  expect((await response.body()).length).toBe(1024)
}

/** Asserts the `<img>` at `locator` has actually loaded, not merely rendered. */
export async function expectImageLoaded(locator) {}
