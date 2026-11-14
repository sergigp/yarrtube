# Tasks

## 1. Walking skeleton

- [x] 1.1 Nothing to create: no new types or signatures. Confirm `cargo build` succeeds and all existing tests pass before starting.

## 2. Behaviour (TDD)

- [x] 2.1 No new behaviour test: the existing `sync_plex_labels_task` suite (incl. `it_should_recreate_a_playlist_deleted_in_plex`) must stay green.

## 3. Infrastructure adapters (TDD)

`HttpPlexLibraryRepository`
- [x] 3.1 `it_should_create_a_smart_playlist_filtered_by_label` (update) — URI lists every labelled episode, no `episode.unwatched`, sorted `originallyAvailableAt:desc,index:desc`; update `smart_playlist_uri` and its doc comment.

## 4. Docs

- [x] 4.1 `doc/CATEGORIES.md` (In Plex): playlists list all a category's episodes, watched ones shown as watched, newest first; same-day episodes of one channel newest first, of different shows in Plex's order.
- [x] 4.2 `doc/PLEX.md` (Watch state): drop "so it leaves your category smart playlists too".

## 5. Verification

- [x] 5.1 `scripts/run-tests.sh --lint --web --rust`.
- [ ] 5.2 Manual (human): after deploying, delete the category playlists in Plex; within 15 minutes they're back with the same titles; "Chess gameplays" lists watched episodes marked watched and a channel's same-day episodes newest first; watching an episode keeps it in the playlist.
