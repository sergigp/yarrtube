# Tasks

## 1. Walking skeleton

- [x] 1.1 In `ytdlp.rs`: add `DOWNLOAD_DEADLINE`, `PROBE_DEADLINE`, `SOCKET_TIMEOUT_SECONDS`; add `deadline` to `YtDlpCommand` and take it in `new(ytdlp_path, deadline)`, passing the per-call deadline from design.md at every call site; rename `output_retrying_busy` to `spawn_retrying_busy` (same ETXTBSY retry, around `spawn`); rewrite `output()` as spawn with `process_group(0)` + two `drain` threads + `wait_with_deadline`, mapping `None` to a timeout `Err` built with `deadline_text`. Stubs: `wait_with_deadline` ignores the deadline and returns `Ok(Some(child.wait()?))`; `deadline_text` returns `format!("{deadline:?}")`; `drain` reads the pipe to its end (needed to keep today's output). Add `libc` to `Cargo.toml` and an empty-for-now `FakeYtDlp::sleeping()` in the test module. No `--socket-timeout` yet. Done when `cargo build` succeeds and all existing tests pass.

## 2. Behaviour (TDD)

None: a timeout surfaces as an `Err` from `ytdlp::*`, the path the existing "yt-dlp cannot run" task tests already cover (see design.md Test Plan).

## 3. Infrastructure adapters (TDD)

`src/infrastructure/shared/ytdlp.rs`:

- [x] 3.1 `it_should_fail_with_a_timeout_if_yt_dlp_outlives_its_deadline` — drives the bounded wait (`recv_timeout`, `killpg`, reap), `deadline_text` and the error message; returns within 5 s for a 1 s deadline.
- [x] 3.2 `it_should_kill_the_children_of_a_timed_out_yt_dlp` — drives killing the whole process group: the fake's own pid and its background child's pid are both gone after the timeout.
- [x] 3.3 `it_should_return_the_whole_output_of_a_yt_dlp_that_exits_in_time` — 1 MiB stdout and stderr survive intact through the concurrent drains, exit code still classifies `Success` / `CleanFailure` (likely passes immediately: keep as regression test).
- [x] 3.4 `it_should_pass_a_socket_timeout_to_yt_dlp` — drives `new` starting the arg list with `--socket-timeout 30`; update any existing exact-args assertions this shifts.

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh --only web --only rust` (fmt, clippy, tests).
- [x] 4.2 Full `scripts/run-tests.sh` (contract + e2e too: this touches the YouTube adapters' real `yt-dlp` invocation). Report any suite that can't run and why.
- [x] 4.3 No commits or pushes unless the user asks (overrides the per-task commit guidance for this change).

## 5. Audit follow-ups

- [x] 5.1 `it_should_kill_the_children_left_by_a_yt_dlp_that_exits_in_time` — drives killing the process group also when `yt-dlp` exits in time (`FakeYtDlp::leaving_a_child`).
- [x] 5.2 `it_should_describe_a_deadline_in_whole_units` — pins the deadline constants and `deadline_text` (surviving mutants).
- [x] 5.3 `Dockerfile`: install `tini` and run it as PID 1; adjust `shutdown_signal`'s PID 1 comment in `src/serve.rs`.
- [x] 5.4 Full `scripts/run-tests.sh` again (the image changed), plus a manual check in the container: `tini` is PID 1, an orphan is reaped, `docker stop` exits cleanly.
