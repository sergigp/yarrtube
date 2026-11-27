interface FavoriteButtonProps {
  videoId: string
  /** Names the video in the unfavorite-kept confirmation. */
  title: string
  favorite: boolean
  /** Set when the video is kept by its source: unfavoriting asks first. */
  kept?: { sourceName: string }
}

/** The video detail pane's star toggle: favorites or unfavorites the video. */
export function FavoriteButton(props: FavoriteButtonProps) {
  void props
  return <></>
}
