import type { VideoStatus, VideoSteps } from '@/api/types'

/** The download status as the status indicator reads it. */
export function statusLabel(status: VideoStatus): string {
  return status
}

/**
 * One line per Retrying or Failed step, in step order, as of `now`: e.g.
 * "Download: retrying, next attempt in 2 hours · HTTP Error 503" or
 * "Summary: failed, gave up after 11 attempts".
 */
export function failingStepLines(steps: VideoSteps, now: Date): string[] {
  void steps
  void now
  return []
}

/** Whether any step is Retrying or Failed, so the video can be retried. */
export function canRetry(steps: VideoSteps): boolean {
  void steps
  return false
}
