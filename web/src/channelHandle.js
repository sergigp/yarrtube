/**
 * Best-effort, client-side derivation of a channel path segment from a raw
 * "Channel Handle or URL" field value: strips a leading `@`, and if the
 * value looks like a URL, takes the handle segment (the first path segment
 * after the host, e.g. `@name` in `youtube.com/@name/videos`) rather than
 * the last one, since a channel URL may carry trailing segments like
 * `/videos` or `/featured`. Mirrors the shape of the backend's
 * `ChannelHandle::from_url_or_handle` without contacting YouTube or
 * replicating its full validation.
 */
export function deriveChannelPathSegment(rawValue) {
  const trimmed = (rawValue ?? '').trim()
  if (!trimmed) {
    return ''
  }

  let candidate = trimmed
  const schemeMatch = trimmed.match(/^https?:\/\//i)
  if (schemeMatch) {
    const afterScheme = trimmed.slice(schemeMatch[0].length)
    const withoutQuery = afterScheme.split(/[?#]/)[0]
    const segments = withoutQuery.split('/').filter(Boolean)
    // segments[0] is the host; the handle is the first path segment after it.
    candidate = segments[1] ?? ''
  }

  return candidate.replace(/^@+/, '')
}
