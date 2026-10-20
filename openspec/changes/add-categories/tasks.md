Prerequisite: `foldable-sidebar-sections` is applied (the Categories heading reuses its fold behaviour).

## 1. Walking skeleton

- [x] 1.1 Create every backend file, type and signature from design.md (migration 0011 registered, `domain/category` module, the six new services plus `list_for_category`/`mark_category_watched`, the new error variants, `SqliteCategoryRepository`, `categories` HTTP module and DTOs, `list_videos_for_category`, routes, `ApiServices` fields, both subscribers registered, `infrastructure_container` and `serve.rs` wiring). Bodies return trivial values (`Ok(())`, empty `Vec`, `None`, `CategoryViews { categories: vec![], unwatched_total: 0 }`, `StatusCode::NO_CONTENT`). Done when `cargo build` succeeds and all existing tests pass; no new tests.
- [x] 1.2 Create every web file, type and signature from design.md (`api/types.ts`, client calls, queries and keys, `lib/categories.ts`, `useSaveSourceCategories`, `aCategory`/`aCategoryList`, `VideoGrid` extracted from `Home.tsx`, `SidebarSectionHeading` extracted from the fold heading, `SidebarCategories`, `AddCategoryDialog`, `CategoriesView`, `CategoryDetail`, `CategoryPicker`, routes in `App.tsx`). Components render a placeholder; nothing new is shown in the sidebar or dialogs yet. Done when `npm run check` passes; no new tests.

## 2. Behaviour (TDD)

Backend acceptance tests:

- [x] 2.1 `it_should_create_a_category` — creating stores a category with a generated id, trimmed name and no members
- [x] 2.2 `it_should_fail_to_create_if_name_missing` — missing name is rejected
- [x] 2.3 `it_should_fail_to_create_if_name_invalid` — `CategoryName` errors map to 400
- [x] 2.4 `it_should_fail_to_create_if_name_taken_ignoring_case` — duplicate name is a conflict
- [x] 2.5 `it_should_list_categories_by_name_ignoring_case` — list order and member lists
- [x] 2.6 `it_should_count_distinct_unwatched_videos_per_category` — per-category count across channel and playlist members, shared video once
- [x] 2.7 `it_should_count_the_total_once_across_categories` — `unwatched_total` is the distinct union
- [x] 2.8 `it_should_skip_members_already_deleted` — a member whose source is gone is neither listed nor counted
- [x] 2.9 `it_should_rename_a_category`
- [x] 2.10 `it_should_rename_a_category_to_a_case_variant_of_its_name`
- [x] 2.11 `it_should_fail_to_rename_if_name_taken`
- [x] 2.12 `it_should_fail_to_rename_a_missing_category`
- [x] 2.13 `it_should_delete_a_category_keeping_its_sources`
- [x] 2.14 `it_should_fail_to_delete_a_missing_category`
- [x] 2.15 `it_should_set_the_members_of_a_category`
- [x] 2.16 `it_should_replace_the_members_of_a_category`
- [x] 2.17 `it_should_keep_a_repeated_member_once`
- [x] 2.18 `it_should_let_a_source_be_in_several_categories`
- [x] 2.19 `it_should_fail_to_set_members_if_a_source_is_not_tracked`
- [x] 2.20 `it_should_fail_to_set_members_if_a_handle_is_invalid`
- [x] 2.21 `it_should_fail_to_set_members_of_a_missing_category`
- [x] 2.22 `it_should_mark_a_category_watched` — every downloaded video of every member and its copies watched, pending ones left
- [x] 2.23 `it_should_fail_to_mark_a_missing_category_watched`
- [x] 2.24 `it_should_list_videos_for_a_category` — across members, newest sync first, with source
- [x] 2.25 `it_should_list_a_shared_video_once_from_the_channel`
- [x] 2.26 `it_should_list_only_downloaded_videos_for_a_category`
- [x] 2.27 `it_should_list_videos_of_a_playlist_excluded_from_home_for_a_category`
- [x] 2.28 `it_should_list_no_videos_for_a_category_without_members`
- [x] 2.29 `it_should_fail_to_list_videos_for_a_missing_category`
- [x] 2.30 `it_should_remove_the_channel_from_every_category` (channel-deleted subscriber)
- [x] 2.31 `it_should_ignore_an_invalid_channel_id` (channel-deleted subscriber)
- [x] 2.32 `it_should_remove_the_playlist_from_every_category` (playlist-deleted subscriber)

Value objects (one cycle each, in `category_name.rs` / `category_id.rs`):

- [x] 2.33 `CategoryName` › accepts a name and trims it
- [x] 2.34 `CategoryName` › rejects empty and blank names
- [x] 2.35 `CategoryName` › accepts 60 characters and rejects 61
- [x] 2.36 `CategoryName` › `is_same_as` ignores case
- [x] 2.37 `CategoryId` › accepts an id and rejects a blank one

Web (Vitest):

- [ ] 2.38 `lib/categories` › orders unread first, most first, ties and caught-up by name ignoring case
- [ ] 2.39 `lib/categories` › finds the categories of a source
- [ ] 2.40 `lib/categories` › computes membership changes: adds, removes, leaves unchanged ones out
- [ ] 2.41 `lib/categories` › adds and removes members without duplicates
- [ ] 2.42 `SidebarCategories` › lists every category above channels with badges, in unread order
- [ ] 2.43 `SidebarCategories` › opens a category and marks its row active
- [ ] 2.44 `SidebarCategories` › manage control navigates without folding
- [ ] 2.45 `SidebarCategories` › folds with the muted unwatched total and marks the heading active on a category route
- [ ] 2.46 `SidebarCategories` › manages and deletes from the row menu, with the member-count confirmation
- [ ] 2.47 `SidebarCategories` › is not filtered by the sidebar search
- [ ] 2.48 `AddCategoryDialog` › creates a category and opens it in the management view
- [ ] 2.49 `AddCategoryDialog` › disables create for a blank name and shows a taken-name error
- [ ] 2.50 `CategoriesView` › lists categories with member counts and selects one from the address
- [ ] 2.51 `CategoriesView` › renames in place, and rejects a taken name keeping the saved one
- [ ] 2.52 `CategoriesView` › groups members by kind and removes one
- [ ] 2.53 `CategoriesView` › filters the picker and adds the checked sources
- [ ] 2.54 `CategoriesView` › deletes the selected category after confirmation
- [ ] 2.55 `CategoriesView` › shows list or details on a narrow viewport
- [ ] 2.56 `CategoryDetail` › shows the members and video cards newest first
- [ ] 2.57 `CategoryDetail` › marks all watched
- [ ] 2.58 `CategoryDetail` › explains an empty category and an unknown one
- [ ] 2.59 `CategoryPicker` › picks, removes and creates categories
- [ ] 2.60 `AddChannelDialog` › adds a channel into the chosen categories, and reports categories that failed to save
- [ ] 2.61 `AddPlaylistDialog` › adds a playlist into the chosen categories
- [ ] 2.62 `EditChannelDialog` › saves a change of categories alone
- [ ] 2.63 `EditPlaylistDialog` › saves a change of categories alone
- [ ] 2.64 `SettingsMenu` › offers Categories
- [ ] 2.65 `useWatchProgress` › refetches categories when a report flips the watched state (other badge-changing actions go through `useInvalidateLibrary`, which now includes categories)

## 3. Infrastructure adapters (TDD)

`SqliteCategoryRepository`:

- [ ] 3.1 `it_should_return_the_category_after_inserting_it`
- [ ] 3.2 `it_should_list_categories_by_name_ignoring_case`
- [ ] 3.3 `it_should_keep_members_in_the_order_first_added`
- [ ] 3.4 `it_should_replace_members_on_update`
- [ ] 3.5 `it_should_delete_the_category_and_its_members`

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass
- [ ] 4.2 `npm run check` and `npm run build` pass in `web/`
- [ ] 4.3 Add `smoke-tests/tests/categories.spec.js` (create, rename, add members, category view, mark all watched, assign from the add channel dialog, delete) and `scripts/run-smoke-tests.sh` passes
- [ ] 4.4 Manual check with `scripts/run-local.sh`: sidebar section order and badges, folding with total, manage view on desktop and mobile, category view, dialogs' Categories field, deleting a channel removes it from its categories
