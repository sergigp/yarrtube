# CLAUDE.md

## What this is

Yarrtube downloads every video in a YouTube playlist to a local directory via
`yt-dlp`. It's packaged as a Docker image meant to run continuously (e.g. on a
NAS), exposing an HTTP API to track playlists plus a `serve` daemon that syncs
them on an interval. See `README.md` for the full deployment/runtime story
(Docker, docker-compose, environment variables, releasing).

## Commands

```bash
cargo build --release                 # build
cargo test --locked                   # run all tests
cargo test <substring>                # run a single test / module by name filter
cargo fmt --all -- --check            # formatting check (CI enforces this)
cargo clippy --all-targets --all-features --locked -- -D warnings   # lint (CI enforces this, warnings fail)
```

## Local testing

For local testing we can run it by running the script locatged in `scripts/run-local.sh`. We can use this for testing our local changes. If you start it, the sqlite database will be created in the repository root `yarrtube.sqlite3` and the videos will be downloaded into `videos/` folder. Necessary env variables should be already loaded in the shell.

Run the binary directly for local (non-Docker) development:

## Spec-driven development (OpenSpec)

This repo uses OpenSpec (`openspec/`) for spec-driven change management —
proposals and specs live under `openspec/changes/` and `openspec/specs/`
(one directory per capability). Use the
`openspec-*` / `opsx:*` skills (propose, apply, update, sync-specs, archive,
explore) when starting, continuing, or finalizing a spec'd change rather than
editing `openspec/` files by hand.
