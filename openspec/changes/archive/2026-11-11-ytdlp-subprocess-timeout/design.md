# Design

## Files

- `src/infrastructure/shared/ytdlp.rs` — the one place every `yt-dlp` process is started; gains deadlines, the bounded wait, the group kill and `--socket-timeout`. Its test module gains `FakeYtDlp::sleeping`.
- `Cargo.toml` — `libc` as a direct dependency (already in `Cargo.lock` transitively), for `killpg`/`kill`.
- `Dockerfile` — installs `tini` and runs it as PID 1 (`ENTRYPOINT ["tini", "--", "docker-entrypoint.sh"]`), so the orphans of a killed group are reaped.
- `src/serve.rs` — `shutdown_signal`'s comment: `yarrtube` is PID 1 only without the image's `tini`.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs` — test only: its fake spots `--simulate` anywhere in the args, now that `--socket-timeout 30` comes first.

## Types & Signatures

```rust
// ytdlp.rs, above YtDlpCommand
/// Run-time limit of a media download.
const DOWNLOAD_DEADLINE: Duration = Duration::from_secs(4 * 60 * 60);
/// Run-time limit of every other invocation (listings, probes, fetches).
const PROBE_DEADLINE: Duration = Duration::from_secs(15 * 60);
/// Passed as `--socket-timeout` to every invocation.
const SOCKET_TIMEOUT_SECONDS: &str = "30";

struct YtDlpCommand<'a> {
    ytdlp_path: &'a Path,
    args: Vec<OsString>,
    working_dir: Option<&'a Path>,
    deadline: Duration,                       // new
}

impl<'a> YtDlpCommand<'a> {
    /// Starts the arg list with `--socket-timeout SOCKET_TIMEOUT_SECONDS`,
    /// so no call site can leave it out.
    fn new(ytdlp_path: &'a Path, deadline: Duration) -> Self;
    fn arg(self, arg: impl AsRef<OsStr>) -> Self;            // unchanged
    fn args<I, S>(self, args: I) -> Self;                     // unchanged
    fn in_dir(self, working_dir: &'a Path) -> Self;           // unchanged
    fn run(&self, purpose: &str) -> Result<RunOutcome>;       // unchanged contract, + timeout Err
    /// Spawns in its own process group, drains stdout/stderr on two threads,
    /// waits up to `deadline`; on expiry SIGKILLs the group, reaps, and
    /// returns `yt-dlp timed out after <deadline> (<purpose>)`.
    fn output(&self, purpose: &str) -> Result<Output>;
    fn describe(&self) -> String;                             // unchanged
}

// free functions, after the existing helpers
/// `output_retrying_busy` renamed: the same ETXTBSY retry, around `spawn`.
fn spawn_retrying_busy(cmd: &mut Command) -> io::Result<Child>;
/// The child's exit status, or `None` if it outlived `deadline`. Either way
/// its whole process group is killed and the child reaped before returning.
fn wait_with_deadline(child: Child, deadline: Duration) -> io::Result<Option<ExitStatus>>;
/// Reads a pipe to its end on its own thread.
fn drain(pipe: impl Read + Send + 'static) -> JoinHandle<Vec<u8>>;
/// What a drain thread read; empty when there was no pipe.
fn joined(handle: Option<JoinHandle<Vec<u8>>>) -> Vec<u8>;
/// `4 hours`, `15 minutes`, `1 second`: whole units, for the timeout error.
fn deadline_text(deadline: Duration) -> String;
```

Deadline per call: `download_video` → `DOWNLOAD_DEADLINE`; `fetch_chapters`, `fetch_thumbnail`, `probe_quality`, `probe_captions`, `fetch_subtitles`, `diagnose`, `list_channel_videos` → `PROBE_DEADLINE`.

The timeout is an `Err` from `output()`, not a `RunOutcome` variant: `CleanFailure` stays "non-zero exit", so a timed-out download skips `diagnose` and can't be classed Unavailable.

## Call Stack

Download (other kinds identical with `PROBE_DEADLINE`):

```
VideoDownloader::run_download
└─ YtDlpVideoDownloaderRepository::download(url, quality, staging)
   └─ ytdlp::download_video(ytdlp_path, url, quality, staging)
      └─ YtDlpCommand::new(ytdlp_path, DOWNLOAD_DEADLINE)   // args: --socket-timeout 30 …
         .run("for <url>")
         └─ output("for <url>")
            ├─ Command { args, stdout/stderr piped, process_group(0), current_dir }
            ├─ spawn_retrying_busy(&mut command)            → Child   (NotFound → not_found_message)
            ├─ drain(child.stdout.take()), drain(child.stderr.take())
            ├─ wait_with_deadline(child, DOWNLOAD_DEADLINE)
            │  ├─ waiter thread: child.wait() → mpsc::Sender<io::Result<ExitStatus>>
            │  ├─ receiver.recv_timeout(deadline)
            │  │   Ok(status)  → Some(status)
            │  │   Timeout     → None
            │  ├─ libc::killpg(pid, SIGKILL)                // either way: no leftover holds the pipes
            │  └─ Timeout: receiver.recv() (reaped)
            ├─ join both drains
            ├─ Some(status) → Output { status, stdout, stderr }
            └─ None         → Err("yt-dlp timed out after 4 hours (for <url>)")
VideoDownloader::record_attempt
└─ Err(e) → record_errored(started, e)   // failed attempt, unchanged code path
```

## Test Plan

1. Behaviour tests: none new. Deadlines are fixed constants not reachable from the task layer in a test; a timeout is an `Err` from `download_video`, the path `download_video_task::it_should_record_the_failure_if_yt_dlp_cannot_run` already covers (failed attempt, reason recorded). Same for the other steps' existing "yt-dlp fails to run" tests.

2. Infrastructure tests — `ytdlp.rs` (real processes, fake `yt-dlp` shell script):
   - `it_should_fail_with_a_timeout_if_yt_dlp_outlives_its_deadline` — `FakeYtDlp::sleeping()` (sleeps 60 s) run through `YtDlpCommand::new(path, 1s).run("for x")` returns `Err` with message `yt-dlp timed out after 1 second (for x)`, within 5 s.
   - `it_should_kill_the_children_of_a_timed_out_yt_dlp` — the sleeping fake starts a background `sleep 60` (stand-in for ffmpeg) and writes its own and the child's pid to files; after the timeout `libc::kill(pid, 0)` fails with `ESRCH` for both (reaped and gone).
   - `it_should_return_the_whole_output_of_a_yt_dlp_that_exits_in_time` — a fake writing 1 MiB to stdout and to stderr (past the pipe buffer) and exiting 3, run with a 10 s deadline, yields `CleanFailure` with the full stderr; a 0 exit yields `Success` with the full stdout.
   - `it_should_kill_the_children_left_by_a_yt_dlp_that_exits_in_time` — `FakeYtDlp::leaving_a_child()` exits 0 at once, leaving a background `sleep 60` that holds the pipes; with a 10 s deadline the call returns `Success` within 5 s and the child is gone.
   - `it_should_describe_a_deadline_in_whole_units` — `deadline_text` of `DOWNLOAD_DEADLINE`, `PROBE_DEADLINE`, 1 s and 90 s is `4 hours`, `15 minutes`, `1 second`, `90 seconds`.
   - `it_should_pass_a_socket_timeout_to_yt_dlp` — `download_video` with `FakeYtDlp::with_exit_code(0)`: `captured_args()` starts with `--socket-timeout`, `30`.
