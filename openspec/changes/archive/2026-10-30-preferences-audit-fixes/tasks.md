# Tasks

## 1. Audit cleanups

- [x] 1.1 Move `useSavedFlash` to `web/src/hooks/useSavedFlash.ts`, use `queryKeys` in `Home.test.tsx`, drop the leftover `TasksView` comment, fix the `savePreferences` import order and the header indentation; `npm run check` passes

## 2. Fixes (TDD)

- [x] 2.1 `PreferencesView` › keeps a second toggle from undoing one still being saved: switches disabled while saving, both changes saved; `npm run test` passes
- [x] 2.2 `Home` › shows no "More about" section while categories are off, even from an older response
- [x] 2.3 `PreferencesView` › reloads cached home videos as soon as a preference is saved (`refetchType: 'all'`)

## 3. Verification

- [x] 3.1 `npm run check`, `cargo test --locked`, fmt, clippy and `scripts/run-tests.sh --only rust --only e2e` pass
