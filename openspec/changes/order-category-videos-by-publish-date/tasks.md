# Tasks

## 1. Walking skeleton

- [x] 1.1 No new types or signatures (private helper `VideoSearcher::published_or_synced` only); `cargo build` succeeds

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_list_category_videos_newest_published_first`: a video published later but recorded earlier is listed first
- [x] 2.2 `it_should_list_videos_for_a_category` (existing, no metadata): videos without a publish time stay in sync-time order

## 3. Infrastructure adapters (TDD)

- [x] 3.1 None: no adapter changes

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh --lint --rust` passes (no web changes)
- [ ] 4.2 Manual (agent): with `scripts/run-local.sh`, a category's grid lists its videos newest published first
