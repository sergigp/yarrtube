/** Known yt-dlp error wordings and what they mean, the first match winning. */
const EXPLANATIONS: readonly [pattern: RegExp, plain: string][] = [
  [/HTTP Error 403/i, 'YouTube refused the request (403)'],
  [/private video/i, 'The video is private'],
  [/confirm your age|age[- ]restricted/i, 'YouTube only shows the video to signed-in adults'],
  [/members[- ]only|join this channel/i, 'The video is for channel members only'],
  [/video unavailable|not available/i, 'YouTube says the video is unavailable'],
]

/**
 * A task's raw error explained in plain words, or, when it matches no known
 * wording, its first line without a leading "ERROR:".
 */
export function explainTaskError(raw: string): string {
  const known = EXPLANATIONS.find(([pattern]) => pattern.test(raw))
  if (known) {
    return known[1]
  }
  const firstLine = raw.split('\n')[0] ?? ''
  return firstLine.replace(/^\s*ERROR:\s*/i, '').trim()
}
