/** Tracks the fixture playlist through the API and waits for its video to download. */
export async function seedDownloadedPlaylist(request) {}

/** Tracks the fixture channel through the API and waits for its videos to download. */
export async function seedDownloadedChannel(request, { videoLimit }) {}

/** Deletes what `seedDownloaded…` tracked and waits for its folder to go. */
export async function deleteSeeded(request, seeded) {}
