/**
 * The lead of the add channel dialog's destination notice, stating how many
 * of the channel's latest videos will be downloaded and naming the channel
 * by its YouTube `title`; the destination path follows it. `videoLimit` is
 * the raw field value, so a limit the dialog would reject leaves the count
 * out rather than stating a wrong one.
 */
export function channelNoticeLead(videoLimit, title) {
  const limit = Number(videoLimit)
  if (String(videoLimit).trim() === '' || !Number.isInteger(limit) || limit < 1 || limit > 1000) {
    return `Videos from “${title}” will be downloaded to`
  }
  if (limit === 1) {
    return `The latest video from “${title}” will be downloaded to`
  }
  return `The latest ${limit} videos from “${title}” will be downloaded to`
}
