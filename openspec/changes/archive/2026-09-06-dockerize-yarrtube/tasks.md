## 1. CLI Restructuring

- [x] 1.1 Add a `download <playlist_url> <output_path>` subcommand that wraps the existing playlist-download flow byte-for-byte (same behavior, only reachable via `download` instead of the bare invocation); verify with `cargo run -- download <url> <dir>` still resolves and downloads a playlist exactly as before
- [x] 1.2 Add empty `serve` and `update-ytdlp` subcommands to the CLI enum (no behavior yet) and verify `cargo run -- serve --help` / `cargo run -- update-ytdlp --help` both print without error and `cargo build` succeeds

## 2. Dependencies

- [x] 2.1 Add `tokio` (with the `rt-multi-thread`, `macros`, `time` features) and `axum` to `Cargo.toml`, scoped to the `serve` subcommand's code path, and verify `cargo build` succeeds
- [x] 2.2 Add `rusqlite` (bundled SQLite) to `Cargo.toml` and verify `cargo build` succeeds

## 3. `update-ytdlp` Task

- [x] 3.1 Implement a function that queries the yt-dlp GitHub releases API for the latest stable release's standalone Linux binary download URL, using the existing `reqwest` blocking client
- [x] 3.2 Implement download-and-replace logic: fetch the binary to a temp file alongside the target path, set it executable, then atomically rename it over the existing yt-dlp binary path, so a failed download never leaves a partial/corrupt binary in place; verify with a unit test that a simulated failure (e.g. write to temp fails) leaves the original file untouched
- [x] 3.3 Wire `update-ytdlp` subcommand to call this logic, printing a clear success/failure message and exiting non-zero on failure; verify by running `cargo run -- update-ytdlp` against a real or stubbed binary path and checking the exit code and message on both a forced success and a forced failure (e.g. unreachable network)
- [x] 3.4 Extract the update logic into a function callable from both the `update-ytdlp` subcommand and `serve`'s startup sequence (task 4.2), so both invocation paths share one implementation; verify by code inspection that no logic is duplicated between the two call sites

## 4. Daemon (`serve`) Startup Sequence

- [x] 4.1 Set up a `tokio` runtime inside the `serve` subcommand handler (`#[tokio::main]` or an equivalent manually-built runtime scoped to that branch), verify `cargo build` succeeds and `serve` starts without panicking
- [x] 4.2 On `serve` startup, call the shared yt-dlp update logic (task 3.4) once; on failure, log the error clearly and continue startup rather than exiting; verify with a test/manual run that a forced update failure (e.g. no network) logs an error and the process keeps running
- [x] 4.3 On `serve` startup, open (creating if missing) a local SQLite file via `rusqlite` and run a trivial query (e.g. `SELECT 1`) to confirm it works; log success or a clearly-labeled database failure; on failure, continue startup rather than exiting; verify by running `serve` and checking the log line, then by forcing a failure (e.g. pointing the path at a non-writable location) and confirming the failure is logged and startup continues
- [x] 4.4 Implement a heartbeat loop (`tokio::time::interval`) that logs a heartbeat message on a recurring interval for the life of the process; verify by running `serve` and observing at least one heartbeat log line after the interval elapses

## 5. HTTP Server and `/status` Endpoint

- [x] 5.1 Start an `axum` HTTP server inside `serve`, listening on a configurable port (env var or CLI flag, with a sensible default), running alongside the heartbeat loop for the life of the process; verify the server accepts a TCP connection while `serve` is running
- [x] 5.2 Add a `GET /status` route that returns `200 OK` with no authentication; verify with `curl -i http://localhost:<port>/status` against a running `serve` process returns HTTP 200

## 6. Docker Packaging

- [x] 6.1 Write a multi-stage `Dockerfile`: build stage `rust:slim-bookworm` compiling the release binary, runtime stage `debian:bookworm-slim` with `ffmpeg` and `ca-certificates` installed via `apt`, copying in the compiled binary; verify `docker build .` succeeds
- [x] 6.2 Set the image's default entrypoint/command to `serve`; verify `docker run <image>` starts the daemon and keeps the container running with no command override
- [x] 6.3 Add a `.dockerignore` (excluding `target/`, `.git/`, `.env`, etc.) and verify the built image does not contain those paths
- [x] 6.4 Verify `download` and `update-ytdlp` can be run against a live container via `docker exec <container> yarrtube download <url> <dir>` / `docker exec <container> yarrtube update-ytdlp`, each exiting with a status reflecting success/failure, without stopping or restarting the daemon
- [x] 6.5 Verify mounting a host directory at the video output path (`docker run -v <host_dir>:<container_path> ...`) makes files written by `download` appear on the host
- [x] 6.6 Verify publishing the HTTP port (`docker run -p <host_port>:<container_port> ...`) makes `/status` reachable from outside the container via `curl`

## 7. Documentation

- [x] 7.1 Update `README.md` to document the container usage: building/running the image, the `download`/`update-ytdlp`/`serve` subcommands, the volume mount for video output, and the published `/status` port
