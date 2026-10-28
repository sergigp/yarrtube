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
  _name: string,
  _unwatchedCount: number,
): MarkWatchedConfirmation | null {
  return null
}
