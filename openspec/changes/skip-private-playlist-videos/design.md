## Files

- `src/infrastructure/repositories/youtube_playlist_items_repository.rs` — the YouTube API adapter: requests `part=snippet,status`, deserializes `status.privacyStatus`, and drops items that are not `public` or `unlisted` before mapping to `YoutubePlaylistItem`. The port, its fake, the domain and the reconciler are unchanged.

## Types & Signatures

```rust
// private API response DTOs in youtube_playlist_items_repository.rs

#[derive(Debug, Deserialize)]
struct PlaylistItem {
    snippet: PlaylistItemSnippet,
    status: Option<PlaylistItemStatus>, // absent => not watchable
}

#[derive(Debug, Deserialize)]
struct PlaylistItemStatus {
    #[serde(rename = "privacyStatus")]
    privacy_status: Option<String>,
}

impl PlaylistItem {
    /// True only for `public` and `unlisted`; private, missing and any
    /// other status (e.g. deleted videos) are not watchable.
    fn is_watchable(&self) -> bool;
}
```

## Call Stack

Reconcile of a YouTube-linked playlist:

```
PlaylistVideoReconciler::sync_playlist_membership(playlist)
  -> YoutubePlaylistItemsRepository::list_current_videos(&playlist_id)
       -> YoutubeApiPlaylistItemsRepository::fetch_page(playlist_id, page_token)
            GET playlistItems?part=snippet,status&maxResults=50&playlistId=..&key=..[&pageToken=..]
            -> parsed.items.into_iter()
                 .filter(PlaylistItem::is_watchable)
                 .map(-> YoutubePlaylistItem { video_id, title, position })
  <- Vec<YoutubePlaylistItem>   (public + unlisted only)
  -> existing diff: new -> stored PENDING; stored but absent -> deleted + VideoRemovedFromPlaylist
```

## Test Plan

Behaviour tests: none. The reconciler's behaviour is unchanged, and the fake port already returns only the items it is given. The existing removed-video tests cover a stored video that is no longer listed.

Infrastructure tests (`YoutubeApiPlaylistItemsRepository`, mockito), in order:

1. `it_should_request_snippet_and_status_parts` — the request carries `part=snippet,status` (mock matches that query param; the call succeeds).
2. `it_should_skip_private_items` — a page with a `public` and a `private` item returns only the public one. The existing tests' fixtures gain `"status": {"privacyStatus": "public"}` in this cycle.
3. `it_should_skip_items_without_a_privacy_status` — an item with no `status` (deleted video) is not returned.
4. `it_should_skip_items_with_an_unrecognised_privacy_status` — an item with `privacyStatus: "privacyStatusUnspecified"` is not returned.
5. `it_should_keep_unlisted_items` — an `unlisted` item is returned with its video id, title and position.
