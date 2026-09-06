## Context

Greenfield repository — no existing Rust code, `Cargo.toml`, or CI yet. This design establishes the initial project shape for the `playlist-download` capability (see proposal.md for motivation). The explicit brief is "prototype, take shortcuts": no persistence, no automation, single sequential path.

## Goals / Non-Goals

**Goals:**
- Stand up a minimal, working Rust binary crate that a human runs manually from a terminal.
- Keep the dependency list small and the code structure simple enough to throw away/rewrite pieces of later without regret.

**Non-Goals:**
- Async/concurrent downloading (spec requires sequential; no design work needed here beyond "don't add a runtime for it").
- Any persistence layer, config file beyond `.env`, or scheduling — explicitly deferred per proposal.md.
- Automated end-to-end tests that hit the real YouTube API or actually invoke `yt-dlp` (would require network/API quota/binary in CI); unit-testable logic (URL/ID parsing, pagination assembly) gets unit tests, the rest is manually verified per tasks.md.

## Decisions

### Single binary crate, no workspace
A plain `cargo init --name yarrtube` binary crate with a few modules (`cli`, `youtube_api`, `downloader`) inside `src/`. No workspace/library split — there's only one entry point and no reuse target yet. Alternative considered: lib+bin split for testability; rejected as premature for a prototype with this little logic.

### `clap` (derive) for argument parsing
Two required positional args: `playlist_url`, `output_path`. `clap` gives free `--help`/usage-error behavior (satisfies the "missing arguments" scenario) for near-zero code. Alternative: hand-rolled `std::env::args()` parsing — rejected, clap costs one dependency and saves boilerplate/edge-case bugs.

### `reqwest` with the `blocking` feature, no `tokio`
The whole program is inherently sequential (one API call round, then N sequential subprocess calls), so an async runtime buys nothing here and would add ceremony (`#[tokio::main]`, `.await` everywhere) for no benefit. `reqwest::blocking` + `serde`/`serde_json` for deserializing `playlistItems.list` responses keeps `main` straight-line. Alternative: `tokio` + async `reqwest` — rejected as unnecessary complexity for a strictly sequential prototype; can revisit if a future change adds concurrent downloads.

### `dotenvy` for `.env` loading
Loads `.env` into the process environment at startup before reading `YOUTUBE_API_KEY`. `.env` is added to `.gitignore`; a `.env.example` (with a placeholder value) is committed so the setup step is documented. Alternative: require the key as a CLI flag — rejected per proposal's explicit requirement to use `.env`.

### Playlist ID extraction: parse `list=` query param
Accept the playlist URL as given (e.g. `https://www.youtube.com/playlist?list=PL...` or a "watch" URL with a `list=` param) and extract the `list` query parameter as the playlist ID to pass to the API. If no `list` param is present, treat it as an invalid playlist URL (per the spec's "invalid playlist" scenario) rather than trying to guess intent.

### `yt-dlp` invocation via `std::process::Command`, inheriting stdio
Each video is downloaded with `Command::new("yt-dlp").arg(video_url).current_dir(output_path)...status()`, with stdout/stderr inherited (not captured) so the user sees `yt-dlp`'s native progress output live — important for a prototype since `yt-dlp` downloads can take a while and silent hangs would look broken. Exit status (not stdout parsing) determines success/failure for the per-video summary.

### Distinguish "binary missing" from "this video failed"
If spawning `yt-dlp` fails outright (`Command::spawn`/`status()` returns an OS-level "not found" error), the design aborts the whole run immediately with a clear error, rather than looping through every remaining video and recording each as a failure — a missing binary is a systemic setup problem, not a per-video one, and the spec's failure-isolation requirement is about individual download failures (private/deleted/region-locked videos etc.), not environment misconfiguration. A non-zero *exit status* from a successfully spawned `yt-dlp` process is treated as a normal per-video failure and does not abort the run.

### Error handling: `anyhow`
`anyhow::Result` throughout `main`/helpers for simple `?`-propagation with context messages, since this is a CLI prototype, not a library other code will match on error types from. Per-video failures are caught and converted to a logged line + summary entry rather than propagated, so one bad video can't unwind the whole run.

## Risks / Trade-offs

- **API quota exhaustion** on very large playlists (each page of 50 items costs quota) → No mitigation in this prototype; acceptable given expected playlist sizes. Future work could cache/resume.
- **Sequential downloads are slow** for large playlists → Accepted trade-off for simplicity and to avoid `yt-dlp`/YouTube rate-limit issues from parallel downloads; explicitly in scope for a future change if needed.
- **No dedup across runs** → Re-running the same playlist re-downloads everything, potentially overwriting/duplicating files depending on `yt-dlp`'s default naming → Accepted per proposal; tracking downloaded videos is explicitly future work.
- **`yt-dlp` version drift** (flags/behavior change over time since we pass no explicit flags) → Accepted; default behavior is exactly what's wanted for the prototype.

## Migration Plan

Not applicable — new project, no existing users or deployed version to migrate from. Initial setup is documented via a committed `.env.example` and a short README section (`cargo build --release`, copy `.env.example` to `.env`, fill in the API key, ensure `yt-dlp` is on `PATH`).
