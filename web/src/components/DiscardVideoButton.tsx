interface DiscardVideoButtonProps {
  videoId: string
  /** Names the video in the confirmation. */
  title: string
  /** Whether the video has files that discarding deletes. */
  downloaded: boolean
}

/**
 * The video detail pane's "Don't download" button: after a confirmation
 * that it can't be undone, discards the video for good.
 */
export function DiscardVideoButton(_props: DiscardVideoButtonProps) {
  return null
}
