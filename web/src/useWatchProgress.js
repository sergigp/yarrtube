import { useEffect, useRef } from 'react'
import { beaconVideoProgress, recordVideoProgress } from './api'

const REPORT_INTERVAL_MS = 15000

/**
 * Attaches to the `videoElement` `<video>` while `video` plays:
 * resumes an unwatched video from its saved position, and reports the
 * position at most every 15s while playing, on pause and end, when the
 * video changes or the view unmounts, and (as a beacon) when the page is
 * hidden or closed. Takes the element itself (from a callback ref) rather
 * than a ref object, so it re-attaches whenever the element mounts later
 * than the video is selected (e.g. while the view is still loading).
 */
export function useWatchProgress(videoElement, video) {
  const latestVideo = useRef(video)
  const videoId = video?.id ?? null
  const playable = video?.status === 'DOWNLOADED' && Boolean(video?.filename)

  useEffect(() => {
    latestVideo.current = video
  })

  useEffect(() => {
    const element = videoElement
    if (!element || !videoId || !playable) {
      return undefined
    }

    let lastReportAt = Date.now()
    let lastReportedPosition = null
    // Kept up to date while playing, since by cleanup time the element may
    // already have switched to another video's source.
    let progress = null

    const capture = () => {
      if (!Number.isFinite(element.currentTime)) {
        return
      }
      progress = {
        position_seconds: Math.floor(element.currentTime),
        duration_seconds: reportableDuration(element.duration),
      }
    }

    const report = (send = recordVideoProgress) => {
      if (!progress || progress.position_seconds === lastReportedPosition) {
        return
      }
      lastReportedPosition = progress.position_seconds
      lastReportAt = Date.now()
      Promise.resolve(send(videoId, progress)).catch(() => {})
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
      report(beaconVideoProgress)
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
    }
  }, [videoElement, videoId, playable])
}

function seekTo(element, seconds) {
  element.currentTime = seconds
}

/**
 * The player's duration in whole seconds, or `undefined` when it isn't known
 * yet (NaN), is unbounded (Infinity), or rounds down to 0 — the daemon only
 * accepts a positive duration.
 */
function reportableDuration(duration) {
  const seconds = Number.isFinite(duration) ? Math.floor(duration) : 0
  return seconds > 0 ? seconds : undefined
}
