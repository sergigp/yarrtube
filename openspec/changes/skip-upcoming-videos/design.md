## Context

The adapters already filter out non-watchable items: the playlist adapter
keeps only public and unlisted items (`PlaylistItem::is_watchable`), and the
reconcilers treat an item missing from the listing as removed. This change
extends that filter in both adapters, and the reconcilers stay unchanged.

## Decisions

- Filtering happens in the adapters and the ports stay unchanged. This
  mirrors the existing `is_watchable`, and both reconcilers already handle an
  item missing from the listing. Positions are assigned after filtering, by
  the `enumerate` in `YtDlpChannelVideosRepository` and by the API's
  `position` for playlists.
- Only `is_upcoming`, `is_live` and `post_live` are skipped. None of these can
  apply to a video that is already downloaded, so a skip never evicts
  downloaded files.
- Playlists: one `videos.list?part=snippet&id=<page ids>` call per
  `playlistItems` page (50 ids or fewer). An id missing from the response is
  kept, because privacy filtering already handles deleted videos.
- `base_url` becomes the API root (`https://www.googleapis.com/youtube/v3`)
  and the adapter appends `/playlistItems` and `/videos`, so tests point both
  at one mockito server.

## Risks / Trade-offs

- [The extra `videos.list` call doubles playlist listing quota] → 1 unit per
  50 items, which is acceptable.
- [An upcoming entry uses one `video_limit` slot on channels] → accepted, and
  it clears once the premiere airs.

## Files

- `src/infrastructure/shared/ytdlp.rs`: `FlatPlaylistEntry` gains
  `live_status`, and `list_channel_videos` drops the entries that aren't
  watchable yet.
- `src/infrastructure/repositories/youtube_playlist_items_repository.rs`:
  per-page `videos.list` lookup, drops upcoming and live items, API-root
  base URL.

## Types & Signatures

```rust
// ytdlp.rs
#[derive(serde::Deserialize)]
struct FlatPlaylistEntry {
    id: String,
    title: String,
    live_status: Option<String>,
}

impl FlatPlaylistEntry {
    /// False for `is_upcoming`, `is_live`, `post_live`.
    fn is_watchable(&self) -> bool;
}
```

```rust
// youtube_playlist_items_repository.rs
const YOUTUBE_API_URL: &str = "https://www.googleapis.com/youtube/v3";

#[derive(Debug, Deserialize)]
struct VideosResponse { items: Vec<VideoItem> }

#[derive(Debug, Deserialize)]
struct VideoItem { id: String, snippet: VideoSnippet }

#[derive(Debug, Deserialize)]
struct VideoSnippet {
    #[serde(rename = "liveBroadcastContent")]
    live_broadcast_content: Option<String>,
}

impl VideoItem {
    /// True for `upcoming` and `live`.
    fn is_broadcast_pending(&self) -> bool;
}

impl YoutubeApiPlaylistItemsRepository {
    /// Ids among `video_ids` that YouTube reports as upcoming or live.
    fn find_not_yet_watchable(&self, video_ids: &[&str]) -> anyhow::Result<HashSet<String>>;
}
```

## Call Stack

Channel reconcile:
`ChannelVideoReconciler::list_current_videos(channel)` →
`YtDlpChannelVideosRepository::list_current_videos(handle, limit)` →
`ytdlp::list_channel_videos(path, url, limit)` → parse each line as a
`FlatPlaylistEntry` → `.filter(FlatPlaylistEntry::is_watchable)` → enumerate
to get positions.

Playlist reconcile:
`YoutubeApiPlaylistItemsRepository::list_current_videos(playlist_id)` → for
each page, `fetch_page(playlist_id, page_token)` →
`GET {base}/playlistItems` → filter with `is_watchable` →
`find_not_yet_watchable(&page_ids)` → `GET {base}/videos?part=snippet&id=a,b,…`
→ drop the returned ids.

## Test Plan

Behaviour: none. Reconciler behaviour is unchanged, and its existing
add/evict tests cover a listing that leaves a video out.

Infrastructure, `ytdlp::list_channel_videos` (fake yt-dlp):
1. `it_should_skip_upcoming_entries_when_listing_channel_videos`: an
   `is_upcoming` line is left out and the other entries are returned in order.
2. `it_should_skip_live_and_post_live_entries_when_listing_channel_videos`:
   `is_live` and `post_live` lines are left out.
3. `it_should_keep_entries_with_a_finished_or_absent_live_status`: `was_live`,
   `not_live` and a missing `live_status` are kept.

Infrastructure, `YoutubeApiPlaylistItemsRepository` (mockito):
4. `it_should_skip_items_reported_as_upcoming`: the `/videos` response marks
   one id `upcoming`, and that item is dropped.
5. `it_should_skip_items_reported_as_live`: an id marked `live` is dropped.
6. `it_should_request_live_status_for_the_page_video_ids`: `/videos` is
   called with `part=snippet` and the page's comma-joined ids.
7. `it_should_keep_items_missing_from_the_videos_response`: an id absent from
   `/videos` is kept.
8. `it_should_fail_when_the_live_status_lookup_fails`: a `/videos` 500 makes
   the listing `Err`.
