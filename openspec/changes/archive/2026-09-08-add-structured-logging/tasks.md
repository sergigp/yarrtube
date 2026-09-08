## 1. Setup

- [x] 1.1 Add `tracing` and `tracing-subscriber` (with the `env-filter`
      feature) to `Cargo.toml` and verify `cargo build --release` succeeds.
- [x] 1.2 In `serve.rs::run()`, install a global `tracing_subscriber::fmt`
      subscriber, filtered by an `EnvFilter` built with
      `.with_default_directive(LevelFilter::INFO.into()).from_env_lossy()`,
      as the first statement — before `run_startup_ytdlp_update()`. Verify
      by running `RUST_LOG=debug ./target/release/yarrtube serve` and
      `./target/release/yarrtube serve` (no var set) and observing leveled
      output in both, with debug-level lines only in the first.

## 2. Convert `serve.rs`

- [x] 2.1 Replace the `println!`/`eprintln!` calls in
      `run_startup_ytdlp_update`, `run_startup_database_check`,
      `run_startup_youtube_api_key_check`, `heartbeat_loop`, `serve_http`,
      `run_async`, and `run` with `tracing::info!`/`warn!`/`error!` per the
      `logging` spec's Leveled Log Output requirement (success → info,
      startup/poll/bind failure → error). Verify by re-running
      `./target/release/yarrtube serve` and confirming every message that
      was previously printed still appears (at `info` default), including
      the final fatal-error paths.

## 3. Convert domain services

- [x] 3.1 Replace the two `println!` calls in
      `domain/playlist/service.rs` (`create_playlist`, `delete_playlist`)
      with `tracing::info!`, attaching `playlist_id` (and `name` where
      available) as structured fields instead of interpolating them into
      the message. Verify with `cargo test --locked playlist::service`
      (or the relevant http tests) still passing and showing no log output
      by default.
- [x] 3.2 Replace the `println!` calls in `domain/video/service.rs`
      (`sync_playlist_videos`: skip / start / video added / video removed /
      next sync scheduled) with `tracing::info!` (or `debug!` for the
      "skipping sync" no-op path — implementer's judgment per design.md),
      attaching `playlist_id`, `video_id`, and `next_run_at` as fields
      where applicable. Verify with `cargo test --locked video::service`
      (or the relevant subscriber/task tests).

## 4. Convert background infrastructure

- [x] 4.1 Replace the `println!`/`eprintln!` calls in
      `infrastructure/repositories/task_executor.rs` (dispatch, poll
      failure, poll panic) with `tracing::info!`/`error!`, attaching
      `task_id`/`task_type` as fields.
- [x] 4.2 Replace the `println!`/`eprintln!` calls in
      `infrastructure/repositories/domain_events_consumer.rs` (dispatch, no
      subscribers, poll failure, poll panic) with
      `tracing::info!`/`warn!`/`error!`, attaching `event_id`/`event_type`
      as fields.
- [x] 4.3 Replace the `println!` calls in
      `infrastructure/repositories/sqlite_task_repository.rs` (scheduled,
      done, retry, permanent failure, recovery) with
      `tracing::info!`/`warn!`/`error!` per the spec's failure-vs-routine
      split, attaching `task_id`/`retries` as fields.
- [x] 4.4 Replace the `println!` calls in
      `infrastructure/repositories/sqlite_event_repository.rs` (published,
      done, retry, permanent failure) with
      `tracing::info!`/`warn!`/`error!`, attaching `event_id`/`retries` as
      fields.
- [x] 4.5 Verify completion of this group: `grep -rn "println!\|eprintln!"
      src/domain src/infrastructure/repositories src/serve.rs` returns no
      matches (the only remaining `println!`/`eprintln!` in the codebase
      are in `cli/download_command.rs`, `cli/ytdlp_update.rs`, and
      `infrastructure/client/youtube_downloader_client.rs`, which are out
      of scope).

## 5. Verify test isolation and ship

- [x] 5.1 Run `cargo test --locked` and confirm no log lines appear in the
      test output (no subscriber is installed anywhere in the test
      binaries, satisfying the `logging` spec's No Log Output in
      Automated Tests requirement with no test code changes).
- [x] 5.2 Run `cargo fmt --all -- --check` and
      `cargo clippy --all-targets --all-features --locked -- -D warnings`
      and fix any findings.
- [x] 5.3 Add a short `RUST_LOG` mention to `README.md`'s environment
      variables section (default `info`, example `RUST_LOG=debug`).
