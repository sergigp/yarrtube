const WHAT_DELETING_DOES: Record<'channel' | 'playlist', string> = {
  channel: 'This removes the channel from tracking.',
  playlist:
    'This removes the playlist from tracking, along with its video records and downloaded files.',
}

/**
 * What deleting a channel or playlist does, for its delete confirmation:
 * saying how many favorite videos it deletes, when it has any.
 */
export function deleteSourceDescription(kind: 'channel' | 'playlist', favoriteCount: number): string {
  const favorites =
    favoriteCount === 0
      ? null
      : favoriteCount === 1
        ? 'Its favorite video will be deleted too.'
        : `Its ${favoriteCount} favorite videos will be deleted too.`
  return [WHAT_DELETING_DOES[kind], favorites].filter(Boolean).join(' ')
}
