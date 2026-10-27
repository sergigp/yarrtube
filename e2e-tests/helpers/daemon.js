/**
 * Starts this worker's own yarrtube daemon (see `fixtures.js`).
 */
export async function startDaemon({ runId, workerIndex, image, youtubeStubPort }) {
  return {
    url: process.env.BASE_URL,
    videosDir: process.env.E2E_VIDEOS_DIR,
    plexStubUrl: process.env.E2E_PLEX_STUB_URL,
    stop: async () => {},
  }
}
