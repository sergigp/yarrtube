/**
 * The app route of a tracked channel or playlist. The id is URL-encoded, since
 * the router decodes it back: an unencoded `%` or `?` in a handle would
 * otherwise reach the page as a different id. A handle's leading `@` is safe
 * in a path, so it is kept readable.
 */
export function channelPath(id: string): string {
  return `/channels/${encodeURIComponent(id).replace(/^%40/, '@')}`
}

export function playlistPath(id: string): string {
  return `/playlists/${encodeURIComponent(id)}`
}
