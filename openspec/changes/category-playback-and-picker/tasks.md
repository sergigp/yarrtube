# Tasks

## 1. Walking skeleton

- [x] 1.1 Add `CategoryVideoView` (`src/domain/video/category_video_view.rs`, re-exported from `mod.rs`), switch `VideoSearcherApi::list_for_category` to `Vec<CategoryVideoView>` (bodies may fill `metadata: None`), add `CategoryVideoResponse` + `From<VideoSource> for HomeVideoSourceResponse` in `dto.rs`, and make `list_videos_for_category` return `Vec<CategoryVideoResponse>`, with the existing category tests asserting through a `category_video_response(..)` helper. On the frontend, add the `CategoryVideo` type and `aCategoryVideo` builder, type `listCategoryVideos`/`useCategoryVideos` as `CategoryVideo[]`, make `VideoListPane` generic over `T extends Video` with `basePathOf`, and have `CategoryPlayer` read the selected video from the category list (dropping `useSourceVideos`). Done when `cargo build`, `cargo test --locked` and `npm run check` pass.

## 2. Behaviour (TDD)

- [x] 2.1 Home card tints at 50% opacity, including the crowded grey (`lib/categoryColors.ts`): verified by `categoryColors.test.ts` and `Home.test.tsx` (already in the working tree).
- [x] 2.2 Categories field as toggle chips in category colors, with a "New category" field that creates or chooses by name (`CategoryPicker.tsx`): verified by `CategoryPicker.test.tsx` and the Add/Edit Channel/Playlist dialog tests (already in the working tree).
- [x] 2.3 `it_should_list_category_videos_with_their_playback_fields`: a category video's entry carries status, quality, file name, sync time and subtitle language. Verified with `cargo test it_should_list_category_videos_with_their_playback_fields`.
- [x] 2.4 `it_should_list_category_videos_with_their_metadata`: a category video's entry carries publish time, description and channel name from its metadata. Verified with `cargo test it_should_list_category_videos_with_their_metadata`.
- [x] 2.5 `it_should_list_category_videos_without_metadata_as_absent`: without generated metadata, those three fields are absent and the rest are still present. Verified with `cargo test it_should_list_category_videos_without_metadata_as_absent`.
- [x] 2.6 Category playback in `CategoryDetail.test.tsx`: a card opens `/categories/:id?video=<id>` and autoplays from its source's path; selecting a queue video from another source plays from that source and shows its channel avatar link; no source video list is requested. Verified with `npm run test -- CategoryDetail`.

## 3. Infrastructure adapters (TDD)

- No adapter changes (metadata uses the existing repository), so nothing to do here.

## 4. Verification

- [x] 4.1 `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo test --locked` and `npm run check` (in `web/`) all pass.
- [x] 4.2 End-to-end: `scripts/run-tests.sh` passes, including the updated picker step in `e2e-tests/tests/categories.spec.js`.
- [x] 4.3 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, seed a category with a channel and a playlist through the HTTP API, and use the Playwright MCP browser to: open the category, click a card, and assert the URL `/categories/<id>?video=<id>`, the `<video>` src under the source path and the queue listing the category's videos; click a queue video from the other source and assert the src changes; open the add channel dialog and assert every category chip is shown, and a pressed one is filled with its color. Take a screenshot of the home grid and of the player. Then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.4 Manual (human): judge visually whether the 50% home tints read as soft enough, and whether the chip colors and outlines stay legible in light and dark themes; check playback, captions and progress resume on a real device in a category queue.
