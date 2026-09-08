# Development guide

## Local setup

1. Copy `.env.example` to `.env` and fill in `YOUTUBE_API_KEY`.
2. Install [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) and make sure it's on
   your `PATH`.
3. Build and run:
   ```bash
   cargo build --release
   ./target/release/yarrtube serve
   ./target/release/yarrtube download <playlist_id> <output_path>
   ./target/release/yarrtube update-ytdlp
   ```

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

## Releasing

Images are built and published automatically by GitHub Actions
(`.github/workflows/release.yml`) — never run `docker build`/`docker push`
by hand. Pushing to `main` only runs CI checks; the image is only built when
a tag is pushed.

```bash
git tag v0.1.0
git push origin v0.1.0
```

This builds a `linux/amd64` image and pushes it to GitHub Container Registry
as both `ghcr.io/sergigp/yarrtube:<version>` and
`ghcr.io/sergigp/yarrtube:latest`.

**One-time setup:** after the first release, the package is private by
default. Make it public (GitHub profile → **Packages** → `yarrtube` →
**Package settings** → **Change visibility** → **Public**) so it can be
pulled without authentication.
