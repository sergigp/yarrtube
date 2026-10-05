# Proposal

## Why

Every merge to `main` is now tagged and released automatically, and the NAS
pulls the new image within minutes — but nothing shows which release is
actually running, so there's no easy way to tell when an update has landed.

## What Changes

- Release builds stamp the release tag (e.g. `v0.9.2`) into the binary at
  compile time; manual `:edge` builds stamp `edge-<short sha>`; builds
  without a version (local `cargo run`) report `dev`. `Cargo.toml`'s version
  is not used, since CI only bumps the tag.
- New `GET /api/version` endpoint returning the running version.
- The web UI shows the version as small, muted text right after the header
  logo (outside the logo's home link), rendering nothing while loading or if
  the request fails.

## Capabilities

### New Capabilities
- `running-version`: identifying which build is running — the version
  stamped into the binary, the endpoint that reports it, and its display in
  the web UI header.

### Modified Capabilities

## Impact

- `.github/workflows/release.yml`: passes the version as a Docker build arg.
- `Dockerfile`: accepts the build arg in the build stage, declared just
  before the final `cargo build` so the dependency cache is kept.
- Rust: a compile-time version constant and a new HTTP handler under
  `src/application/http/`.
- Web: API type, client call and hook for the version; label in the header
  in `web/src/App.tsx`.
- Out of scope (possible follow-up): showing uptime/start time, and telling
  an open tab that a new version is running.
