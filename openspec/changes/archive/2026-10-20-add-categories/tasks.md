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

- [x] 2.38 `lib/categories` › orders unread first, most first, ties and caught-up by name ignoring case
- [x] 2.39 `lib/categories` › finds the categories of a source
- [x] 2.40 `lib/categories` › computes membership changes: adds, removes, leaves unchanged ones out
- [x] 2.41 `lib/categories` › adds and removes members without duplicates
- [x] 2.42 `SidebarCategories` › lists every category above channels with badges, in unread order
- [x] 2.43 `SidebarCategories` › opens a category and marks its row active
- [x] 2.44 `SidebarCategories` › manage control navigates without folding
- [x] 2.45 `SidebarCategories` › folds with the unwatched total badge and marks the heading active on a category route
- [x] 2.46 `SidebarCategories` › manages and deletes from the row menu, with the member-count confirmation
- [x] 2.47 `SidebarCategories` › is not filtered by the sidebar search
- [x] 2.48 `AddCategoryDialog` › creates a category and opens it in the management view
- [x] 2.49 `AddCategoryDialog` › disables create for a blank name and shows a taken-name error
- [x] 2.50 `CategoriesView` › lists categories with member counts and selects one from the address
- [x] 2.51 `CategoriesView` › renames in place, and rejects a taken name keeping the saved one
- [x] 2.52 `CategoriesView` › groups members by kind and removes one
- [x] 2.53 `CategoriesView` › filters the picker and adds the checked sources
- [x] 2.54 `CategoriesView` › deletes the selected category after confirmation
- [x] 2.55 `CategoriesView` › shows list or details on a narrow viewport
- [x] 2.56 `CategoryDetail` › shows the members and video cards newest first
- [x] 2.57 `CategoryDetail` › marks all watched
- [x] 2.58 `CategoryDetail` › explains an empty category and an unknown one
- [x] 2.59 `CategoryPicker` › picks, removes and creates categories
- [x] 2.60 `AddChannelDialog` › adds a channel into the chosen categories, and reports categories that failed to save
- [x] 2.61 `AddPlaylistDialog` › adds a playlist into the chosen categories
- [x] 2.62 `EditChannelDialog` › saves a change of categories alone
- [x] 2.63 `EditPlaylistDialog` › saves a change of categories alone
- [x] 2.64 `SettingsMenu` › offers Categories
- [x] 2.65 `useWatchProgress` › refetches categories when a report flips the watched state (other badge-changing actions go through `useInvalidateLibrary`, which now includes categories)

## 3. Infrastructure adapters (TDD)

`SqliteCategoryRepository`:

- [x] 3.1 `it_should_return_the_category_after_inserting_it`
- [x] 3.2 `it_should_list_categories_by_name_ignoring_case`
- [x] 3.3 `it_should_keep_members_in_the_order_first_added`
- [x] 3.4 `it_should_replace_members_on_update`
- [x] 3.5 `it_should_delete_the_category_and_its_members`

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass
- [x] 4.2 `npm run check` and `npm run build` pass in `web/`
- [x] 4.3 Add `smoke-tests/tests/categories.spec.js` (create, rename, add members, category view, mark all watched, assign from the add channel dialog, delete) and `scripts/run-smoke-tests.sh` passes
- [x] 4.4 Manual check with `scripts/run-local.sh`: sidebar section order and badges, folding with total, manage view on desktop and mobile, category view, dialogs' Categories field, deleting a channel removes it from its categories

## 5. Rework: manage from the category view

Replaces the management view (tasks 2.44, 2.48, 2.50–2.56, 2.64 and their code) with management on the category view, per the revised spec and design.

- [x] 5.1 Skeleton: add the shadcn `toggle` and `toggle-group` wrappers; create `ManageSourcesDialog`, `EditableTitle`, `ManageSourcesState` and the new `lib/categories` functions as stubs; widen `DetailHeader` (`onSync` optional, `summaryPrefix`, `onRename`, `menuItems`, `videos` any array). Remove `CategoriesView` (+ test), the `/categories` route, the Settings menu "Categories" item (+ its test), and the sidebar heading's manage control and the `SidebarSectionHeading` actions slot (+ their tests). Point every former `/categories?selected=` link at the category view. Done when `npm run check` passes.

Web (Vitest, TDD):

- [x] 5.2 `lib/categories` › builds chips per kind by name ignoring case, channels with avatars
- [x] 5.3 `lib/categories` › filters chips by search and state keeping their order
- [x] 5.4 `lib/categories` › toggles a source and summarises staged additions and removals
- [x] 5.5 `EditableTitle` › renames on Enter and on blur, cancels on Escape, closes unchanged without saving
- [x] 5.6 `EditableTitle` › keeps the field open with the message when the name is rejected
- [x] 5.7 `DetailHeader` › shows the summary prefix and menu items, and no Sync without `onSync`
- [x] 5.8 `CategoryDetail` › shows the summary and video cards newest first
- [x] 5.9 `CategoryDetail` › marks all watched and deletes from the actions menu
- [x] 5.10 `CategoryDetail` › renames from the header
- [x] 5.11 `CategoryDetail` › explains an empty category, whose control opens the dialog, and an unknown one
- [x] 5.12 `CategoryDetail` › opens the dialog on arrival with `ManageSourcesState`, once
- [x] 5.13 `ManageSourcesDialog` › shows channels and playlists as chips pressed by membership
- [x] 5.14 `ManageSourcesDialog` › stages toggles, summarises them and saves the full set at once
- [x] 5.15 `ManageSourcesDialog` › filters by search and state, and Enter toggles the only match
- [x] 5.16 `ManageSourcesDialog` › disables Save with nothing staged and asks before discarding
- [x] 5.17 `ManageSourcesDialog` › keeps staged changes when saving fails
- [x] 5.18 `ManageSourcesDialog` › says when nothing is tracked
- [x] 5.19 `SidebarCategories` › "Manage sources" in the row menu opens the category view with the dialog
- [x] 5.20 `AddCategoryDialog` › creates a category and opens it with the manage sources dialog

Verification:

- [x] 5.21 `npm run check` and `npm run build` pass in `web/`
- [x] 5.22 Update `smoke-tests/tests/categories.spec.js` (create lands in manage sources, add sources with chips, rename in place, mark all watched, delete from the actions menu) and `scripts/run-smoke-tests.sh` passes
- [x] 5.23 Manual check with `scripts/run-local.sh`: category header and menu, rename, the dialog on desktop and mobile (chips, search, filters, keyboard, discard), entry points from the sidebar row, empty state and add category

## 6. Sidebar add controls and folded badge

Per the revised Sidebar Add Entries, Folded Section Unwatched Total and Sidebar Categories Section requirements. Marking a whole section watched from its heading is out of scope.

- [x] 6.1 `SidebarSectionHeading` takes an optional `add` control (trailing "+" outside the `h3`); Channels, Playlists and Categories show the full-width add row only when loaded and empty, otherwise the heading "+"
- [x] 6.2 `Sidebar` › offers a labelled add row in an empty section and a heading + once it has items
- [x] 6.3 Folded total rendered as the row badge; `Sidebar` › shows the channels' unwatched total badge only while folded and above 0 (renamed), and 2.45 renamed to match
- [x] 6.4 `npm run check` passes in `web/`
- [x] 6.5 Smoke helpers find a section's list, rows and messages as siblings of its heading row (the `h3` is now wrapped with the "+"), in one place in `helpers/sidebar.js`
- [x] 6.6 `smoke-tests/tests/categories.spec.js` › category on mobile: add from the drawer, manage sources full screen, then add from the heading
- [x] 6.7 `scripts/run-smoke-tests.sh` passes
- [x] 6.8 Manual check with `scripts/run-local.sh`: heading "+" and empty add rows in all three sections, folded badges, on desktop and mobile
