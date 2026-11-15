/** What to ask before marking a channel, playlist or category watched. */
export interface MarkWatchedConfirmation {
  title: string
  description: string
}

/**
 * The confirmation every "Mark all watched" asks first, since it has no
 * undo. `unwatchedCount` must be the real count, also when it is hidden.
 */
export function markWatchedConfirmation(
  name: string,
  unwatchedCount: number,
): MarkWatchedConfirmation {
  return {
    title:
      unwatchedCount === 1
        ? `Mark 1 video in "${name}" watched?`
        : unwatchedCount > 1
          ? `Mark ${unwatchedCount} videos in "${name}" watched?`
          : `Mark everything in "${name}" watched?`,
    description: "This can't be undone in bulk: videos can only be marked unwatched one by one.",
  }
}
