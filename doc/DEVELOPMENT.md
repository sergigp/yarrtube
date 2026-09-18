# Development guide

*In progress...*

## Local setup

1. Copy `.env.example` to `.env` and fill in `YOUTUBE_API_KEY`.
2. Install [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) and set up `YTDLP_PATH` in `.env` file.
3. Run the script located in `./scripts/run_local.sh

## Testing & linting

```bash
cargo test --locked
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
```

CI (`.github/workflows/ci.yml`) runs all three on every PR and push to
`main`.

## Architecture

See `CLAUDE.md` and `.claude/skills/rust-architect/SKILL.md` for the
project's layering, naming, and testing conventions.
