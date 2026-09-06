## 1. Project Setup

- [x] 1.1 Run `cargo init --name yarrtube` at the repo root and verify `cargo build` succeeds with the default generated `main.rs`
- [x] 1.2 Add `clap` (derive), `reqwest` (`blocking`, `json` features), `serde`/`serde_json` (derive), `dotenvy`, and `anyhow` to `Cargo.toml`, and verify `cargo build` still succeeds after adding them
- [x] 1.3 Add `.env` to `.gitignore` and commit a `.env.example` containing `YOUTUBE_API_KEY=your-api-key-here`, and verify `git status` shows `.env` as ignored when a local `.env` file is created
- [x] 1.4 Add a short README section documenting setup (`cp .env.example .env` + fill in key, ensure `yt-dlp` is on `PATH`, `cargo build --release`) and verify the documented steps work end-to-end on a clean checkout

## 2. CLI Argument Parsing

- [x] 2.1 Define a `clap`-derived struct with two required positional args (`playlist_url`, `output_path`) and verify `yarrtube` with zero/one arguments prints a usage error and exits non-zero (spec: CLI Invocation - Missing arguments)
- [x] 2.2 Verify `yarrtube <url> <path>` parses successfully and the parsed values are accessible in `main` (spec: CLI Invocation - Valid invocation)

## 3. Configuration Loading

- [x] 3.1 Load `.env` via `dotenvy::dotenv()` at the start of `main` and read `YOUTUBE_API_KEY`, and verify running with a valid `.env` proceeds past the config step
- [x] 3.2 On missing/empty `YOUTUBE_API_KEY`, print a clear error and exit non-zero before any network call, and verify by unsetting/removing the key and confirming no HTTP request is attempted (spec: API Key Configuration - Key missing or empty)

## 4. YouTube Playlist Resolution

- [x] 4.1 Implement playlist ID extraction from the input URL by parsing the `list` query parameter, and add unit tests covering a `playlist?list=...` URL, a `watch?v=...&list=...` URL, and a URL with no `list` param (expect an error) (spec: Playlist Resolution)
- [x] 4.2 Define `serde` structs for the relevant fields of the YouTube Data API v3 `playlistItems.list` response (video id/title, `nextPageToken`)
- [x] 4.3 Implement a function that calls `playlistItems.list` for a given playlist ID and API key and returns one page of `(video_url, title)` pairs plus an optional next page token
- [x] 4.4 Implement pagination: loop calling 4.3 following `nextPageToken` until exhausted, combining all pages into one ordered list, and add a unit test (with a fake/mocked paging function) verifying multi-page results are combined in order (spec: Playlist Resolution - Multi-page playlist)
- [x] 4.5 On an API error response (invalid/inaccessible playlist), print the error and exit non-zero without invoking `yt-dlp`, and verify manually with a bogus playlist URL (spec: Playlist Resolution - Invalid or inaccessible playlist)
- [x] 4.6 On a valid playlist with zero videos, print a "no videos found" message and exit zero without invoking `yt-dlp`, and verify manually with an empty playlist (spec: Playlist Resolution - Empty playlist)

## 5. Video Download

- [x] 5.1 Implement output directory creation (`fs::create_dir_all`) before downloads start, and verify manually that a non-existent output path is created (spec: Sequential Video Download - Output directory does not exist)
- [x] 5.2 Implement a function that invokes `yt-dlp <video_url>` via `std::process::Command` with `current_dir` set to the output path and stdio inherited, returning success/failure based on exit status
- [x] 5.3 If spawning `yt-dlp` fails (binary not found), print a clear setup error and abort the entire run immediately, and verify manually by temporarily renaming/removing `yt-dlp` from `PATH` (design: "Distinguish binary missing from this video failed")
- [x] 5.4 Loop over the resolved video list sequentially, invoking 5.2 for each one, logging a failure line (title + URL) and continuing to the next video when `yt-dlp` exits non-zero, without aborting the run (spec: Sequential Video Download, Per-Video Failure Isolation)
- [x] 5.5 After the loop, print a summary of total succeeded/failed counts and list the titles/URLs of any failed videos, exiting non-zero if any failed and zero otherwise, and verify manually with a playlist containing at least one unavailable/private video mixed with valid ones (spec: Per-Video Failure Isolation - all scenarios)

## 6. End-to-End Verification

- [x] 6.1 Manually run `yarrtube <real playlist url> <tmp dir>` against a small real playlist (mix of public videos, and ideally one private/unavailable one) and verify: all valid videos are downloaded into the target directory, failures are logged individually, the run does not abort early, and the final summary matches what happened
