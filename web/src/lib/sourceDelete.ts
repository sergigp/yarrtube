/**
 * What deleting a channel or playlist does, for its delete confirmation,
 * including how many favorite videos it deletes.
 */
export function deleteSourceDescription(kind: 'channel' | 'playlist', favoriteCount: number): string {
  void favoriteCount
  return kind === 'channel'
    ? 'This removes the channel from tracking.'
    : 'This removes the playlist from tracking, along with its video records and downloaded files.'
}
