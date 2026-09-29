/**
 * The lead of the add playlist dialog's destination notice, stating how many
 * videos from which playlist will be downloaded; the destination path
 * follows it. `videoCount` is YouTube's own count, so an empty playlist
 * leaves the count out rather than promising nothing.
 */
export function playlistNoticeLead(videoCount, title) {
  if (videoCount === 0) {
    return `Videos from “${title}” will be downloaded to`
  }
  if (videoCount === 1) {
    return `The only video from “${title}” will be downloaded to`
  }
  return `All ${videoCount} videos from “${title}” will be downloaded to`
}
