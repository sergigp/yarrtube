/**
 * Best-effort, client-side derivation of a channel path segment from a raw
 * "Channel Handle or URL" field value: strips a leading `@`, and if the
 * value looks like a URL, takes its last non-empty path segment first.
 * Mirrors the shape of the backend's `ChannelHandle::from_url_or_handle`
 * without contacting YouTube or replicating its full validation.
 */
export function deriveChannelPathSegment(rawValue) {
  const trimmed = (rawValue ?? '').trim()
  if (!trimmed) {
    return ''
  }

  let candidate = trimmed
  if (/^https?:\/\//i.test(trimmed)) {
    const withoutQuery = trimmed.split(/[?#]/)[0]
    const segments = withoutQuery.split('/').filter(Boolean)
    candidate = segments[segments.length - 1] ?? ''
  }

  return candidate.replace(/^@+/, '')
}
