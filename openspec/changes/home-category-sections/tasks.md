# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md across all layers: `HomeCategorySection`, `HomeVideos.categories`, `HomeLimits.category_sections`/`category_videos` (`HOME_LIMITS`: 3 / 6), `category_sections_in` / `pending_in` returning empty `Vec`, `category_ids_by_video` taking the listed categories, `HomeCategorySectionResponse` + `HomeResponse.categories`, web `HomeCategorySection` / `HomeVideos.categories`, `aHomeCategorySection` builder, existing Home test fixtures given `categories: []`. Done when `cargo build` and `npm run check` succeed and all existing tests pass. No new behaviour, no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_list_a_category_section_with_its_pending_videos_on_home` — a category's pending videos become a section with id, name, color, newest sync first
- [x] 2.2 `it_should_list_no_category_sections_if_nothing_pending_on_home` — watched videos are not pending; no section
- [x] 2.3 `it_should_cap_a_category_section_at_6_on_home` — section holds the 6 most recent
- [x] 2.4 `it_should_list_a_category_section_video_once_from_the_channel_on_home` — once per YouTube video, channel copy
- [x] 2.5 `it_should_not_repeat_a_continue_watching_video_in_category_sections_on_home` — continue watching wins
- [x] 2.6 `it_should_not_repeat_a_category_section_video_in_quick_watches_on_home` — sections sit above quick watches
- [x] 2.7 `it_should_not_repeat_a_category_section_video_in_latest_on_home` — sections sit above latest
- [x] 2.8 `it_should_rank_category_sections_by_pending_videos_on_home` — most pending first
- [x] 2.9 `it_should_count_every_pending_video_when_ranking_category_sections_on_home` — full backlog counted, not the 6 shown
- [x] 2.10 `it_should_not_count_continue_watching_videos_when_ranking_category_sections_on_home` — ranking excludes continue watching
- [x] 2.11 `it_should_break_category_section_ties_by_name_on_home` — ties by name ignoring case
- [x] 2.12 `it_should_cap_category_sections_at_3_on_home` — at most 3 sections
- [x] 2.13 `it_should_list_a_video_in_two_categories_in_the_first_section_only_on_home` — no repeats across sections
- [x] 2.14 `it_should_skip_a_category_emptied_by_earlier_sections_on_home` — emptied category skipped, next takes its place
- [x] 2.15 `it_should_leave_videos_of_a_playlist_excluded_from_home_out_of_category_sections_on_home` — excluded playlists never feed sections
- [x] 2.16 `it_should_show_a_category_video_through_a_source_shown_on_home` — video reachable through a non-excluded source still shows
- [x] 2.17 Web: `shows a "More about" section per category between continue watching and quick watches` — headings in response order between CW and QW
- [x] 2.18 Web: `navigates to the category from the section heading` — heading links to `/categories/:id`
- [x] 2.19 Web: `navigates to the category from "See all"` — "See all" links to `/categories/:id`
- [x] 2.20 Web: `shows no "More about" section when the response has none`

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 None — no adapter changes (confirm nothing in `src/infrastructure/` needed touching)

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `npm run check` (in `web/`) all pass
- [ ] 4.2 End-to-end tests via `scripts/run-tests.sh` pass
- [ ] 4.3 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos` (a pre-existing repo-root `yarrtube.sqlite3` is the user's — do not seed into or delete it; move it aside first and restore it after, or ask), seed channels/playlists and 4 categories via the HTTP API with differing pending counts, open home with the Playwright MCP browser and assert: section order CW → "More about" ×3 (most pending first) → Quick watches → Latest; no video id repeated across sections (DOM + `GET /api/videos/home` response); heading and "See all" navigate to `/categories/:id`; screenshot. Stop the server, delete the seeded `yarrtube.sqlite3` and the fresh videos dir, restore the user's DB
- [ ] 4.4 Manual (human): visual judgement of the "More about" heading + "See all" row on desktop and phone widths
