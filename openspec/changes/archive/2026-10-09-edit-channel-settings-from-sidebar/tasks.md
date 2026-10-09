## 1. Walking skeleton

- [x] 1.1 Create `hooks/useSaveChannelSettings.ts` with the save flow moved from `ChannelDetail`, switch `ChannelDetail` to it, and thread `onEditRequest` through `SidebarSection` → `SidebarRow` → `SidebarRowMenu` with `Sidebar` owning `editingChannelId` and rendering `EditChannelDialog`; done when `npm run typecheck` passes and existing tests pass

## 2. Behaviour (TDD)

- [x] 2.1 `EditChannelDialog` opens titled "Edit <name> settings" (update the prefill test, and `ChannelDetail.test.tsx` dialog names); verify the dialog tests pass
- [x] 2.2 `EditChannelDialog` notes that a changed quality applies to new videos only; verify the new test passes
- [x] 2.3 `EditChannelDialog` shows no quality note while the quality is unchanged; verify the new test passes
- [x] 2.4 `Sidebar` channel row menu lists Sync, Mark all watched, Edit settings, Delete; verify the updated test passes
- [x] 2.5 `Sidebar` edits a channel's quality from its row menu, sending only the quality and no reconcile; verify the new test passes
- [x] 2.6 `Sidebar` syncs a channel after its video limit changes from its row menu; verify the new test passes
- [x] 2.7 `Sidebar` playlist rows offer no Edit settings; verify the updated test passes

## 3. Infrastructure adapters (TDD)

- [x] 3.1 None: no adapter changes; verify `git diff --stat src/` is empty

## 4. Verification

- [x] 4.1 `npm run check` passes in `web/`; `cargo test --locked`, `cargo fmt --all -- --check` and clippy still pass
- [ ] 4.2 Smoke test matches the dialog by `/^Edit .+ settings$/`; run `scripts/run-smoke-test.sh` (or `scripts/run-local.sh` and check by hand: sidebar ⋮ → Edit settings, quality note, title)
