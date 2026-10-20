import { useEffect, useRef } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { beaconVideoProgress, recordVideoProgress } from '@/api/client'
import { queryKeys } from '@/api/queries'
import type { Video, VideoProgress } from '@/api/types'

const REPORT_INTERVAL_MS = 15000

/**
 * Attaches to the `videoElement` `<video>` while `video` plays:
 * resumes an unwatched video from its saved position, and reports the
 * position at most every 15s while playing, on pause and end, when the
 * video changes or the view unmounts, and (as a beacon) when the page is
 * hidden or closed. Takes the element itself (from a callback ref) rather
 * than a ref object, so it re-attaches whenever the element mounts later
 * than the video is selected (e.g. while the view is still loading).
 * When a non-beacon report comes back with a watched state different from
 * the last known one, it refetches the channel, playlist and category lists,
 * so unwatched badges follow a video crossing the watched threshold without
 * refetching on every report.
 * Playback is tracked in sessions: one begins when a video attaches, and
 * every report states whether the video was watched then. A report that
 * changes the watched state (playback crossing the watched threshold, or a
 * rewatch resetting it) carries the session on under the new state. When
 * `video`'s watched state changes under a session otherwise (e.g. it was
 * marked watched), a new session begins: the player pauses and rewinds, and
 * the held position is dropped, so a position from before the change is
 * never reported. A change seen while a report is pending waits for its
 * answer, since the list may show the report's own change first.
 */
export function useWatchProgress(videoElement: HTMLVideoElement | null, video: Video | null): void {
  const queryClient = useQueryClient()
  const latestVideo = useRef(video)
  const sessionRef = useRef<PlaybackSession | null>(null)
  const videoId = video?.id ?? null
  const watched = Boolean(video?.watched)
  const playable = video?.status === 'DOWNLOADED' && Boolean(video?.filename)

  useEffect(() => {
    latestVideo.current = video
  })

  useEffect(() => {
    const element = videoElement
    if (!element || !videoId || !playable) {
      return undefined
    }

    const startedWatched = Boolean(latestVideo.current?.watched)
    const session: PlaybackSession = {
      watched: startedWatched,
      lastWatched: startedWatched,
      // Kept up to date while playing, since by cleanup time the element may
      // already have switched to another video's source.
      progress: null,
      lastReportedPosition: null,
      pendingReport: null,
    }
    sessionRef.current = session
    let lastReportAt = Date.now()

    const capture = () => {
      if (!Number.isFinite(element.currentTime)) {
        return
      }
      session.progress = {
        position_seconds: Math.floor(element.currentTime),
        duration_seconds: reportableDuration(element.duration),
        was_watched: session.watched,
      }
    }

    const report = ({ beacon = false } = {}) => {
      const { progress } = session
      if (!progress || progress.position_seconds === session.lastReportedPosition) {
        return
      }
      session.lastReportedPosition = progress.position_seconds
      lastReportAt = Date.now()
      if (beacon) {
        beaconVideoProgress(videoId, progress)
        return
      }
      const pendingReport = recordVideoProgress(videoId, progress)
        .then(({ watched: nowWatched }) => {
          if (nowWatched !== session.lastWatched) {
            session.watched = nowWatched
            session.lastWatched = nowWatched
            queryClient.invalidateQueries({ queryKey: queryKeys.channels, exact: true })
            queryClient.invalidateQueries({ queryKey: queryKeys.playlists, exact: true })
            queryClient.invalidateQueries({ queryKey: queryKeys.categories, exact: true })
          }
        })
        .catch(() => {})
        .finally(() => {
          if (session.pendingReport === pendingReport) {
            session.pendingReport = null
          }
        })
      session.pendingReport = pendingReport
    }

    const resume = () => {
      const current = latestVideo.current
      if (current?.id === videoId && !current.watched && current.position_seconds > 0) {
        seekTo(element, current.position_seconds)
      }
    }

    const onTimeUpdate = () => {
      capture()
      if (!element.paused && Date.now() - lastReportAt >= REPORT_INTERVAL_MS) {
        report()
      }
    }

    const onStop = () => {
      capture()
      report()
    }

    const onPageHide = () => {
      capture()
      report({ beacon: true })
    }

    if (element.readyState >= HTMLMediaElement.HAVE_METADATA) {
      resume()
    }
    element.addEventListener('loadedmetadata', resume)
    element.addEventListener('timeupdate', onTimeUpdate)
    element.addEventListener('pause', onStop)
    element.addEventListener('ended', onStop)
    window.addEventListener('pagehide', onPageHide)

    return () => {
      element.removeEventListener('loadedmetadata', resume)
      element.removeEventListener('timeupdate', onTimeUpdate)
      element.removeEventListener('pause', onStop)
      element.removeEventListener('ended', onStop)
      window.removeEventListener('pagehide', onPageHide)
      report()
      sessionRef.current = null
    }
  }, [videoElement, videoId, playable, queryClient])

  useEffect(() => {
    const session = sessionRef.current
    if (!videoElement || !session || session.watched === watched) {
      return
    }
    const restart = () => {
      const nowWatched = Boolean(latestVideo.current?.watched)
      if (sessionRef.current !== session || session.watched === nowWatched) {
        return
      }
      // A new session: the held position predates the change, so it's dropped
      // and playback rewinds to a start that is never reported on its own. The
      // rewind comes before the pause, so the pause reports nothing.
      session.watched = nowWatched
      session.lastWatched = nowWatched
      session.progress = null
      session.lastReportedPosition = 0
      seekTo(videoElement, 0)
      videoElement.pause()
    }
    if (session.pendingReport) {
      session.pendingReport.then(restart)
    } else {
      restart()
    }
  }, [videoElement, watched])
}

/**
 * What a playback session tracks: whether the video was `watched` when it
 * began or since its own reports changed that (sent with every report), the
 * watched state last known from a report, the position held for the next
 * report, the last one reported and the report still awaiting its answer.
 */
interface PlaybackSession {
  watched: boolean
  lastWatched: boolean
  progress: VideoProgress | null
  lastReportedPosition: number | null
  pendingReport: Promise<void> | null
}

function seekTo(element: HTMLVideoElement, seconds: number): void {
  element.currentTime = seconds
}

/**
 * The player's duration in whole seconds, or `undefined` when it isn't known
 * yet (NaN), is unbounded (Infinity), or rounds down to 0 — the daemon only
 * accepts a positive duration.
 */
function reportableDuration(duration: number): number | undefined {
  const seconds = Number.isFinite(duration) ? Math.floor(duration) : 0
  return seconds > 0 ? seconds : undefined
}
