## Why

Every merge to `main` that passes CI is currently tagged `vX.Y.Z` and pushed
as `:latest`, so every user picks up untested changes as soon as they merge.
The maintainer tests on their own NAS, after the merge. The NAS needs
continuous delivery. Other users need releases that were cut on purpose.

## What Changes

- **BREAKING (release process)**: merges to `main` no longer create a git tag
  or move `:latest`. Every merge that passes CI publishes `:edge` and
  `:sha-<short sha>` instead. The auto-tag CI job and
  `scripts/next-release-tag.sh` are removed.
- A stable release happens only when the maintainer pushes a `vX.Y.Z` tag
  (e.g. via `gh release create vX.Y.Z --generate-notes`). It publishes
  `:latest`, `:X.Y.Z` and `:X.Y`.
- The tag is the version: `Cargo.toml`'s version isn't read by anything and
  is never bumped (a comment there says so).
- A manual run of the release workflow publishes only `:sha-<short sha>`, so
  testing a branch image never overwrites `:edge`.
- An older edge build can never overwrite `:edge` with an older commit.
- Images carry the commit they were built from (full sha, subject line,
  commit date). `GET /api/version` returns it along with the build channel
  (`release` / `edge` / `dev`).
- Web UI header: release builds show the version as today. Edge builds show
  an `edge` badge. Clicking it opens a popover with the short sha (linked to
  the commit on GitHub), the commit subject and the commit date.
- Docs:
  - New "Deploy" section in `doc/DEVELOPMENT.md`: edge CD, rollback by
    pinning `:sha-<short>`, and how to cut a stable release.
  - `doc/INSTALLATION.md` documents the image tags.
  - The release paragraph in `CLAUDE.md` is updated.

## Capabilities

### New Capabilities
- `release-channels`: which image tags CI publishes on a merge to `main`, on
  a pushed release tag and on a manual run.

### Modified Capabilities
- `running-version`:
  - Edge builds come from every merge, not only manual runs.
  - The version endpoint also returns channel and commit.
  - The header shows an edge badge with commit details.

## Impact

- `.github/workflows/ci.yml`: the `tag` job is removed; the call to
  `release.yml` becomes the edge publish.
- `.github/workflows/release.yml`: `channel` input, tag rules,
  commit build args.
- `scripts/next-release-tag.sh`: deleted.
- `Dockerfile`: new commit build args.
- Backend:
  - `src/infrastructure/shared/app_version.rs`
  - `src/application/http/mod.rs` (`RunningVersion` state)
  - `src/application/http/version/{mod,dto}.rs`
  - `src/serve.rs`
- Frontend:
  - `web/src/api/types.ts` (`AppVersion`)
  - fixture builder in `web/src/test/helpers.tsx`
  - `web/src/components/VersionLabel.tsx`
  - a new `ui/popover.tsx` (radix)
  - `AppShell.test.tsx`
- Docs: `doc/DEVELOPMENT.md`, `doc/INSTALLATION.md`, `CLAUDE.md`.
- `Cargo.toml`: a comment marking its version as unused.
- Ops: the NAS switches its 5-minute pull from `:latest` to `:edge`.
