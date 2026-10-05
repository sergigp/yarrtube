## 1. Walking skeleton

- [x] 1.1 Create the structure from design.md with trivial bodies, then commit. In `lib/tasks.ts`, set the new `TaskFamily`/`TaskTab`/`TASK_TABS` types and add `TaskKind`, `taskKind` (returns `'maintenance'`), `SyncTarget` and `syncTarget` (returns `null`). In `api/queries.ts`, add `useInvalidateTasks`. In `TasksView.tsx`, add the `TAB_ICON`/`KIND_ICON` maps, switch to `TabsList variant="line"`, and add a private `RunNowButton` that is rendered nowhere yet. Leave the backend untouched. Remove the old-family/Cleanup/All test cases that no longer type-check; tasks 2.5–2.17 rewrite them. Verify: `cargo build`, `cargo test --locked` and `npm run check` all pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_include_playlist_id_in_reconcile_playlist_tasks`: the playlist reconcile task view carries `playlist_id` next to `playlist_name`. Remove the superseded `it_should_include_playlist_name_in_reconcile_playlist_tasks`. Commit.
- [x] 2.2 `it_should_keep_playlist_id_if_playlist_is_gone`: the id stays in the payload when the playlist can't be found. It replaces `it_should_omit_details_if_playlist_is_gone`. Commit.
- [x] 2.3 `it_should_include_channel_id_in_reconcile_channel_tasks`: the channel reconcile task view carries `channel_id` next to `channel_name`. Remove the superseded `it_should_include_channel_name_in_reconcile_channel_tasks`. Commit.
- [x] 2.4 `it_should_keep_channel_id_if_channel_is_gone`: the id stays when the channel can't be found. It replaces `it_should_omit_details_if_channel_is_gone`. Commit.
- [x] 2.5 `maps each task type to downloads, syncs or other, and unknown types to other`: drives `FAMILY_BY_TYPE` and the `other` fallback. Commit.
- [x] 2.6 `lists only playlist and channel reconciles in the syncs tab`: Plex sync is no longer in Syncs. Commit.
- [x] 2.7 `collects Plex sync, yt-dlp update, deletions and unknown types in the other tab`: drives `tasksForTab(tasks, 'other')`. Commit.
- [ ] 2.8 `returns counts where downloads, syncs and other add up to every task`: drives `tabCounts` over the new tabs. Commit.
- [ ] 2.9 `maps each task type to its icon kind`: drives `taskKind` (download, sync, delete, maintenance). Commit.
- [ ] 2.10 `returns a sync target for pending playlist and channel reconciles only`: drives `syncTarget`, which returns null for a running task, a Plex sync or a missing id. Commit.
- [ ] 2.11 `shows Active, Downloads, Syncs and Other tabs with counts, and no Cleanup or All tab`: drives the tab triggers in TasksView. Commit.
- [ ] 2.12 `shows a live indicator on the Active tab only while a task is running`: drives the Beacon beside the Active count. Commit.
- [ ] 2.13 `runs a playlist sync now from the Syncs tab and refetches the tasks`: render `RunNowButton` on Syncs rows; on success it calls `reconcilePlaylist` and then invalidates the tasks and library queries. Commit.
- [ ] 2.14 `runs a channel sync now from the Syncs tab`: drives the channel branch through `reconcileChannel`. Commit.
- [ ] 2.15 `disables Run now while the sync is in progress`: the button is disabled with a spinning icon while the request is pending. Commit.
- [ ] 2.16 `shows the failure on the row and lets Run now be retried`: shows the error inline and re-enables the button. Commit.
- [ ] 2.17 `offers no Run now on a running sync or outside the Syncs tab`: guards on the tab and on `syncTarget`. Commit.
- [ ] 2.18 Restyle the `line` variant in `ui/tabs.tsx` and the TasksView tab bar to meet the "Tasks View Tab Bar Affordance" requirement. This is a visual task, not a TDD cycle:
  - primary underline on the selected tab
  - heavier full-colour label when selected, muted label otherwise
  - hover background
  - icon and count pill, with a primary pill on the selected tab
  - tabs ≥40px tall and sized to their content
  - focus ring
  - horizontal scroll when the tabs overflow

  Verify: `npm run check` passes, and checking the page in a browser in light and dark themes at desktop and phone widths matches the spec scenarios. Commit.

## 3. Infrastructure adapters (TDD)

No adapter changes in this change, so this section has no tasks.

## 4. Verification

- [ ] 4.1 Run `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `npm run check`; all must pass.
- [ ] 4.2 Check manually with `scripts/run-local.sh`:
  - the tasks view shows Active, Downloads, Syncs and Other with correct counts
  - Plex, yt-dlp and cleanup tasks appear under Other
  - Run now on a playlist sync and on a channel sync completes, refreshes the list and leaves that task's scheduled time unchanged
  - the tab bar looks clickable in both themes and scrolls on a narrow viewport
