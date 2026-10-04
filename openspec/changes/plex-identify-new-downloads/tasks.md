## 1. Metadata before media

- [x] 1.1 Add `VideoDownloaderRepository::prepare_folder` (wrapping `ytdlp::prepare_video_dir`), pass the resolved folder to `download` as its `existing_folder`, and have the downloader remove a freshly prepared folder when the download does not succeed (fakes updated); verify the existing download/thumbnail-ahead tests plus new fresh/reused-folder cleanup tests pass with `cargo test download_video`
- [x] 1.2 Add `VideoMetadataRepository::write_nfo` (writes `movie.nfo` only, no DB row) and a `remove_nfo` best-effort delete; verify with repository tests asserting the file is written/removed and `find` returns `None`
- [x] 1.3 Reorder `VideoDownloader::download`: fetch metadata + position → prepare folder → `write_nfo` with `thumb` = `<folder>.jpg` → `yt-dlp` → on success `save` (re-fetching only if the first fetch failed), on failure `remove_nfo`; verify with downloader tests for every `video-metadata` scenario (nfo present before the fake downloader writes media, thumb dropped when no thumbnail, fetch-fails-before-succeeds-after, download fails → no row and no nfo, fetch fails both times → no nfo)

## 2. Folder scan on download

- [x] 2.1 Add `DomainEvent::VideoDownloaded { video_id, output_dir, folder }` with its serialization, and give `VideoDownloader` an event publisher that publishes it after a successful, recorded download (not for a video deleted mid-download); verify with downloader tests on the published events
- [x] 2.2 Add to `PlexCollectionRepository` `list_sections()` returning `PlexSection { id, locations }` (`GET /library/sections`) and `scan_path(section_id, path)` (`GET /library/sections/{id}/refresh?path=`), with fake support; verify with mockito tests on request path, query encoding and token header
- [ ] 2.3 Add a pure Plex-path mapping function (container folder + videos root + Plex videos root → `Option<plex path>`, path-segment-aware prefix checks) and the section selection by location prefix; verify with unit tests covering inside/outside root, trailing slashes and sibling-prefix (`/videos2`) cases
- [ ] 2.4 Add subscriber `scan_plex_folder_on_video_downloaded` (scan each configured section containing the path; no section → warn and succeed; outside root → debug and succeed; Plex error → `Err` so the event retries); verify with subscriber tests for each `plex-collections` "Downloaded videos are scanned into Plex" scenario
- [ ] 2.5 Read `YARRTUBE_PLEX_VIDEOS_PATH` in `serve.rs` (non-empty only) and register the subscriber only when Plex is enabled and it is set; extend `it_should_register_the_subscribers_of_every_event_type` so `VideoDownloaded` is covered in both configurations

## 3. Self-heal unidentified items

- [ ] 3.1 Make `PlexItem.youtube_video_id` an `Option<String>` and have `list_items` return every item (collection item listing keeps only identified ones); verify with the existing repository tests plus one for an item with only a `local://` guid
- [ ] 3.2 Add `list_match_candidates(rating_key)` (`GET /library/metadata/{key}/matches`) and `match_item(rating_key, guid, name)` (`PUT /library/metadata/{key}/match`) with fake support; verify with mockito tests using the real response shape captured in design.md
- [ ] 3.3 In `PlexCollectionReconciler`, re-match each unidentified section item to its NFO YouTube candidate before converging collections (info on match, warn on no candidate or failure, never fail the pass, no requests when all items are identified); verify with reconciler tests for each "Unidentified Plex items are re-matched" scenario, and that "Pass is idempotent" still holds

## 4. Timeout, docs, release checks

- [ ] 4.1 Raise `REQUEST_TIMEOUT` in `plex_collection_repository.rs` to 30 s and update its comment; verify `cargo test plex` passes
- [ ] 4.2 Document `YARRTUBE_PLEX_VIDEOS_PATH` (meaning, NAS example `/volume1/data/media/yarrtube`, what is skipped without it) in `README.md` env table, `doc/PLEX.md` and the compose example; verify by reading the rendered docs
- [ ] 4.3 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `cargo test --locked`; all must pass
- [ ] 4.4 After deploying with `YARRTUBE_PLEX_VIDEOS_PATH` set: confirm in `yarrlogs` that the first sync pass logs re-matches for the 10 `local://` items, that a new download logs a folder scan, and that Plex collection counts equal yarrtube's downloaded counts (Bob el manetes after a one-off forced scan)
