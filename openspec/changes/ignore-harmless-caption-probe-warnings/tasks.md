## 1. Walking skeleton

- [x] 1.1 Rename `warning_lines` to `caption_warning_lines` in `src/infrastructure/shared/ytdlp.rs`, add `is_harmless_probe_warning` returning `false`, call it as a filter, update the `CaptionProbe.warnings` and `log_inconclusive` doc comments; done when `cargo build` succeeds and all existing tests pass

## 2. Behaviour (TDD)

- [x] 2.1 No new behaviour tests (application behaviour unchanged per design.md Test Plan); verify `cargo test fetch_subtitles` passes

## 3. Infrastructure adapters (TDD)

### ytdlp::probe_captions

- [x] 3.1 `it_should_drop_a_cache_write_warning_from_a_probe` — cache write failure warning is filtered out; test passes
- [x] 3.2 `it_should_drop_a_formats_skipped_warning_from_a_probe` — `formats have been skipped` warning is filtered out; test passes
- [x] 3.3 `it_should_keep_a_caption_warning_next_to_harmless_ones_in_a_probe` — only the caption warning remains; test passes

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass
