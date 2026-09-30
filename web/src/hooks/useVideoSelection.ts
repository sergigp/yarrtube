import { useState } from 'react'
import { useSearchParams } from 'react-router-dom'
import type { Video } from '@/api/types'

export interface VideoSelection {
  /** The video the player shows, or `null` while none can be. */
  selectedVideo: Video | null
  /** Autoplay only a deep-linked video: the user opened a link straight to it. */
  autoplay: boolean
  /** Selects a video from the list by its ID. */
  selectVideo: (id: string) => void
}

/**
 * Which of a detail view's `videos` is selected: an explicit pick from the
 * list wins, then the `?video=` deep link, then the first (latest) video.
 * The selection is kept as an ID and resolved against the current list, so
 * a poll refreshing `videos` never resets it.
 */
export function useVideoSelection(videos: Video[] | undefined): VideoSelection {
  const [searchParams] = useSearchParams()
  const [manualSelectionId, setManualSelectionId] = useState<string | null>(null)

  const manualSelection = manualSelectionId
    ? (videos?.find((video) => video.id === manualSelectionId) ?? null)
    : null
  const initialVideoId = searchParams.get('video')
  const deepLinkedVideo =
    !manualSelection && initialVideoId
      ? (videos?.find((video) => video.id === initialVideoId) ?? null)
      : null
  const defaultVideo = !manualSelection && !deepLinkedVideo ? (videos?.[0] ?? null) : null
  const selectedVideo = manualSelection ?? deepLinkedVideo ?? defaultVideo
  const autoplay = selectedVideo !== null && selectedVideo === deepLinkedVideo

  return { selectedVideo, autoplay, selectVideo: setManualSelectionId }
}
