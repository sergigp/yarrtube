/** What to ask before marking a channel, playlist or category watched. */
export interface MarkWatchedConfirmation {
  title: string
  description: string
}

/**
 * The confirmation to show before marking `name` watched, or null to mark it
 * right away. `unwatchedCount` must be the real count, also when it is hidden.
 */
export function markWatchedConfirmation(
  name: string,
  unwatchedCount: number,
): MarkWatchedConfirmation | null {
  // One video is no worse than a single video's "Mark as watched", which never asks.
  if (unwatchedCount <= 1) {
    return null
  }
  return {
    title: `Mark ${unwatchedCount} videos in "${name}" watched?`,
    description: "This can't be undone in bulk: videos can only be marked unwatched one by one.",
  }
}
