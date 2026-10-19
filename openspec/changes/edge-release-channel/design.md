## Context

- `ci.yml` has a `tag` job: on a push to `main` it runs `scripts/next-release-tag.sh`, pushes `vX.Y.Z`, and then calls `release.yml` through `workflow_call`. The call is needed because tags pushed with `GITHUB_TOKEN` don't trigger `push: tags`.
- `release.yml` chooses its image tags based on `github.event_name`:
  - `latest` when the event is `push`
  - `edge` + `sha` when the event is `workflow_dispatch`
- In a `workflow_call`, `github.event_name` is the **caller's** event (`push`). That's why CI-called builds get `:latest` today. Reusing the call for edge builds without changing this would also tag them `:latest`.
- The version is a compile-time `option_env!("YARRTUBE_VERSION")` in `app_version.rs`, passed as a Docker `ARG` after `cargo chef cook`. It ends up in the `RunningVersion(String)` axum state, which `GET /api/version` serializes.
- `Cargo.toml` declares `0.9.0`. The latest tag is `v0.9.4`, because CI only bumped tags.

## Goals / Non-Goals

**Goals:** choose the image tags from an explicit channel instead of the event, carry commit metadata end to end, keep the release flow to a few commands.

**Non-Goals:**
- Changelog tooling beyond `gh release create --generate-notes`.
- Pre-release tags (`-rc`).
- A `:X` major tag (meaningless while on `0.x`).
- Configuring Watchtower or the NAS (documented only).

## Decisions

**1. Explicit `channel` input on `release.yml` (`edge` | `release` | `sha`).**
- The channel is resolved in the first job:
  - `workflow_call` → `inputs.channel`
  - `push: tags` → `release`
  - `workflow_dispatch` → `sha`
- Every tag rule and the version computation read the resolved channel. `github.event_name` is no longer used for this.
- *Alternative:* a separate `edge.yml` workflow. Rejected because it would duplicate the two-runner build/merge pipeline.

**2. CI calls `release.yml` with `channel: edge` and passes no tag.**
- The `tag` job and `next-release-tag.sh` are deleted.
- `:edge` only moves when the build's commit is still `main`'s head at manifest-push time (`gh api repos/<repo>/commits/main`, checked in `merge`). A build that finishes after a newer merge landed still publishes its `:sha-<short>` but leaves `:edge` alone.
- *Alternative (rejected during apply):* `concurrency: { group: edge-publish, cancel-in-progress: true }` on the CI job. Concurrency doesn't know commit order. If an older merge's CI finishes after a newer one's, its publish starts later, cancels the newer one, and `:edge` ends up on the older commit.
- The CI run-level `cancel-in-progress: false` stays as it is.

**3. Version gate as a first job (`verify-version`) in `release.yml`, only for `channel == release`.**
- It compares `${TAG#v}` with `Cargo.toml`'s `version` (same `sed` as the old script).
- On a mismatch it prints both versions and exits 1. `build` needs this job, so nothing is pushed.
- *Alternative:* derive the version from `CARGO_PKG_VERSION` and ignore the tag. Rejected because the tag is what users see on ghcr and in the UI, so the two must agree.

**4. Commit metadata as build args, read with `option_env!`.**
- New Docker `ARG`s, declared next to `YARRTUBE_VERSION` after `cargo chef cook`:
  - `YARRTUBE_COMMIT_SHA`
  - `YARRTUBE_COMMIT_SUBJECT`
  - `YARRTUBE_COMMIT_DATE` (from `git log -1 --format=%s` / `%cI` on the checked-out ref)
- Only the subject line is used, which keeps `build-args` (newline-separated) safe.
- The commit **date** is used rather than the build time: it is deterministic and is what "when was this merged" means.
- `app_version.rs` exposes `AppVersion { version, channel, commit: Option<Commit> }`.
  - The channel is derived, not passed in: `dev` when `YARRTUBE_VERSION` is unset, `edge` when it starts with `edge-`, otherwise `release`.
  - `commit` is `Some` only when all three commit vars are set.
- `RunningVersion` becomes the state that wraps this value. `VersionResponse` mirrors it, with `channel` serialized lowercase and `commit` as `null` when absent.

**5. Web: extend `AppVersion` in `types.ts` and add an `aVersion` builder in `helpers.tsx`.**
- `VersionLabel` branches on `channel`:
  - `edge` → a `Badge` that triggers a new shadcn-style `ui/popover.tsx` (radix `Popover`, already available through `radix-ui`).
  - otherwise → the existing muted text.
- Radix Popover handles click, touch, keyboard, Escape and outside-click.
- The popover shows:
  - a short-sha `<a target="_blank" rel="noreferrer">` to `https://github.com/sergigp/yarrtube/commit/<sha>`
  - the subject
  - `formatDateTime(date)` from `src/lib/formatDateTime.ts`
- The repo URL is a constant in the component. There's no need to ship it from the backend.
- *Alternative:* a hover tooltip (`ui/tooltip.tsx`). Rejected because hover doesn't work on touch devices.

## Risks / Trade-offs

- [The first stable release is blocked because `Cargo.toml` (0.9.0) is behind the tags (v0.9.4)] → the docs and tasks say to bump to `0.9.5`+ in the PR before tagging.
- [Users on `:latest` stop getting fixes until the maintainer cuts a release] → intended. The Deploy docs describe cutting a release as two commands.
- [A cancelled edge publish leaves orphan untagged digests on ghcr] → harmless storage. Ignored.
- [A commit subject with characters that break YAML or a shell] → it's passed through `env:` and `$GITHUB_ENV` and never interpolated into a `run:` script.

## Migration Plan

1. Merge the change. Its own merge publishes the first `:edge`. `:latest` stays on `v0.9.4`.
2. On the NAS, point the image and the 5-minute pull at `:edge`.
3. To release: bump `Cargo.toml` to `0.9.5` in a PR, merge it, then run `gh release create v0.9.5 --generate-notes --target main`.
4. Rollback of a bad edge build: pin the NAS to `:sha-<previous>` until a fix merges. Rollback of the release process itself: revert the PR.
