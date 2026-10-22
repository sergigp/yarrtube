/** Asserts the video at `relativePath` (under the videos root) is H.264/AAC in `.mp4`. */
export async function expectH264Mp4(relativePath) {}

/** Asserts `mediaUrl` answers a byte-range request with a partial response. */
export async function expectRangeSupported(request, mediaUrl) {}

/** Asserts the `<img>` at `locator` has actually loaded, not merely rendered. */
export async function expectImageLoaded(locator) {}
