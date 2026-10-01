## 1. Walking skeleton

- [x] 1.1 In `lib/tasks.ts` add the signatures from design.md (`TaskFamily`, `TaskTab`, `TASK_TABS`, `TASK_SEARCH_THRESHOLD`, `taskFamily`, `tasksForTab`, `tabCounts`, `matchesTask`) with trivial stub bodies, and rewrite `TasksView.tsx` to render the five tabs (default `active`) with a stub search field and row (family icon, `describeTask`, badges, `last_error` line) wired end-to-end. Verify `npm run typecheck` passes and the existing `npm run test` suite still passes. No new tests in this task.

## 2. Component behaviour (TDD)

- [x] 2.1 `TasksView` opens on the Active tab listing only running tasks — verify the test fails first, then passes.
- [x] 2.2 Each tab trigger shows its task count — verify the test passes.
- [x] 2.3 Selecting Downloads lists download/thumbnail tasks and no sync/cleanup tasks — verify the test passes.
- [x] 2.4 An empty Active tab shows "Nothing running right now" while other tabs still list tasks — verify the test passes.
- [x] 2.5 The search field is hidden at ≤15 tasks and shown above 15 — verify the test passes.
- [x] 2.6 Typing in search filters the current tab by description, and a non-matching term shows "Nothing matches" — verify the test passes.
- [x] 2.7 Switching tabs clears the search text — verify the test passes.
- [x] 2.8 A retried task's `last_error` is shown on its row and a task with no error shows none — verify the test passes.

## 3. Pure-logic unit tests (TDD)

- [x] 3.1 `describeTask` describes a `reconcile_plex_collections` task in plain language — verify the test fails first, then passes.
- [x] 3.2 `describeTask` describes an `update_ytdlp` task in plain language — verify the test passes.
- [x] 3.3 `describeTask` never returns a raw snake_case string for an unknown type (humanized fallback) — verify the test passes.
- [x] 3.4 `taskFamily` maps each task type to its family — verify the test passes.
- [x] 3.5 `tasksForTab('active', …)` returns only running tasks of any type — verify the test passes.
- [x] 3.6 `tasksForTab` returns the right types for `downloads`, `syncs`, and `cleanup` — verify the test passes.
- [x] 3.7 `tasksForTab('all', …)` returns every task sorted by `byCategory` — verify the test passes.
- [x] 3.8 A `maintenance` task appears in `all` and (when running) `active`, but not in the other family tabs — verify the test passes.
- [x] 3.9 `tabCounts` returns counts equal to each tab's listed length — verify the test passes.
- [x] 3.10 `matchesTask` matches on the description case-insensitively — verify the test passes.

## 4. Verification

- [x] 4.1 Run `npm run check` (typecheck + lint + test) in `web/` and confirm it passes.
- [ ] 4.2 Manually verify in the running app via `scripts/run-local.sh`: tabs, counts, default Active, search, per-row descriptions and last error all behave as specified.
