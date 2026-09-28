## 1. Walking skeleton

- [x] 1.1 In `youtube_playlist_items_repository.rs`, add `PlaylistItemStatus`, the `status: Option<PlaylistItemStatus>` field on `PlaylistItem`, and `PlaylistItem::is_watchable` returning `true`, and wire `.filter(PlaylistItem::is_watchable)` into `fetch_page` before the mapping. The request query is unchanged. Done when `cargo build` succeeds and all existing tests pass

## 2. Behaviour (TDD)

_None: reconciler behaviour is unchanged (see design.md Test Plan)._

## 3. Infrastructure adapters (TDD)

### YoutubeApiPlaylistItemsRepository

- [ ] 3.1 `it_should_request_snippet_and_status_parts`: the request sends `part=snippet,status`
- [ ] 3.2 `it_should_skip_private_items`: a `private` item is dropped and the `public` item beside it is returned. Add `"status": {"privacyStatus": "public"}` to the existing tests' fixtures so they keep passing
- [ ] 3.3 `it_should_skip_items_without_a_privacy_status`: an item with no `status` (deleted video) is dropped
- [ ] 3.4 `it_should_skip_items_with_an_unrecognised_privacy_status`: an item with `privacyStatus: "privacyStatusUnspecified"` is dropped
- [ ] 3.5 `it_should_keep_unlisted_items`: an `unlisted` item is returned with its video id, title and position

## 4. Verification

- [ ] 4.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `cargo test --locked`, and verify all pass
- [ ] 4.2 Run `scripts/run-local.sh` against a playlist containing a private video. Verify that no "Private video" entry is stored or downloaded, and that an existing stuck "Private video" row is removed on the first reconcile pass
