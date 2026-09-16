## 1. Domain: thread the source's storage path

- [x] 1.1 Change `VideoSource` (`src/domain/video/recent_video.rs`) from `Playlist(PlaylistId)` / `Channel(ChannelHandle)` to `Playlist(PlaylistId, PlaylistPath)` / `Channel(ChannelHandle, PlaylistPath)`. Update `VideoSearcher::list_recent`'s `recent_from_playlists`/`recent_from_channels` (`src/domain/services/video_searcher.rs`) to pass `playlist.path.clone()` / `channel.path.clone()` (already in scope from the fetched `Playlist`/`Channel`) alongside the id. Verify it compiles.

## 2. HTTP: surface path and thumbnail on the recent-videos response

- [x] 2.1 Add `path: String` to `RecentVideoSourceResponse` and `thumbnail_filename: Option<String>` to `RecentVideoResponse` (`src/application/http/videos/dto.rs`), populated from the new `VideoSource` data and `recent_video.video.thumbnail_filename`. Verify with unit tests on the `From<RecentVideo>` conversion covering: a playlist source's path is included, a channel source's path is included, thumbnail filename present, thumbnail filename absent (serializes as `null`/omitted consistent with `VideoResponse`'s existing null handling).
- [x] 2.2 Extend the `list_recent_videos` handler tests (`src/application/http/videos/mod.rs`) covering the combined/mixed-source scenario to also assert `source.path` and `thumbnail_filename` in the JSON response.

## 3. SPA: render thumbnails

- [x] 3.1 Add `.list-item-thumbnail` and a placeholder variant to `web/src/App.css`: a fixed-size (e.g. 48x27, 16:9) box with `object-fit: cover` and rounded corners for the `<img>`, and a muted/empty box of the same dimensions for the placeholder, both left of the title in a list row.
- [x] 3.2 Update `Home.jsx` to render a thumbnail `<img src={videoMediaUrl(video.source.path, video.thumbnail_filename)}>` when `video.thumbnail_filename` is present, else the placeholder box, before the title in each card. Verify by checking the rendered markup for both a thumbnailed and a non-thumbnailed video.
- [x] 3.3 Apply the same treatment to the sidebar video list rows in `PlaylistDetail.jsx` (using `playlist.path`) and `ChannelDetail.jsx` (using `channel.path`) — no backend change needed there since `GET .../videos` already returns `thumbnail_filename`. Verify by checking the rendered markup for both cases in each component.

## 4. Verification

- [x] 4.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`; all must pass.
- [x] 4.2 Run `npm run lint` and `npm run build` in `web/`; both must pass.
- [x] 4.3 Manually exercise the full flow against a running daemon with at least one downloaded video that has a thumbnail and one that doesn't (in a playlist or channel): confirm thumbnails render as `<img>` in Home cards and sidebar lists, and the thumbnail-less video shows the placeholder box with the row still aligned.
