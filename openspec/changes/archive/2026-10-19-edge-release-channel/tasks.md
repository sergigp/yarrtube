## 1. Backend version metadata

- [x] 1.1 In `src/infrastructure/shared/app_version.rs`:
  - Replace `APP_VERSION` with an `AppVersion { version, channel, commit: Option<Commit { sha, subject, date }> }`, built from `YARRTUBE_VERSION` and `YARRTUBE_COMMIT_{SHA,SUBJECT,DATE}` via `option_env!`.
  - Derive the channel: `dev` when unset, `edge` when prefixed `edge-`, else `release`.
  - Keep the comment saying Cargo.toml's version is unused.
  - Verify: unit tests cover derivation from explicit inputs (release, edge, dev), and an unset build reports `dev` with no commit.
- [x] 1.2 Make `RunningVersion` in `src/application/http/mod.rs` carry `AppVersion`, and wire it in `src/serve.rs`. Verify: `cargo build` passes.
- [x] 1.3 Extend `VersionResponse` in `src/application/http/version/dto.rs` with `channel` (lowercase) and `commit` (`null` when absent), and update the handler test in `version/mod.rs`. Verify: whole-response asserts for an edge build and a dev build pass.

## 2. Image build and workflows

- [x] 2.1 `Dockerfile`: declare `ARG YARRTUBE_COMMIT_SHA`, `YARRTUBE_COMMIT_SUBJECT` and `YARRTUBE_COMMIT_DATE` next to `YARRTUBE_VERSION`, after `cargo chef cook`. Verify: a local `docker build --build-arg ...` reports them via `GET /api/version`.
- [x] 2.2 `release.yml`:
  - Add a `channel` input to `workflow_call` and resolve the channel: `push: tags` → `release`, `workflow_dispatch` → `sha`.
  - Compute the version (tag, or `edge-<short>`) and the commit sha/subject/date from the checked-out ref into `$GITHUB_ENV`, and pass all of them as `build-args`.
  - Tag rules by channel:
    - `release` → `latest`, `{{version}}`, `{{major}}.{{minor}}`
    - `edge` → `edge`, `sha`
    - `sha` → `sha` only
  - Verify: `actionlint` passes (if available), and a review shows no tag rule still reads `github.event_name`.
- [x] 2.3 ~~`release.yml`: add a `verify-version` job comparing the tag with `Cargo.toml`.~~ Obsolete: implemented, then removed in `4b690ed` (the tag is the version, see design decision 3). Instead, a comment in `Cargo.toml` marks its version as unused. Verify: `release.yml` has no `verify-version` job.
- [x] 2.4 `ci.yml`:
  - Delete the `tag` job.
  - Make `release` call `release.yml` with `channel: edge`, `if: push to main`, `needs: [openspec-archived, ci]`.
  - Edge-never-backwards is enforced in `release.yml`'s `merge` job: `:edge` moves only while the commit is still `main`'s head (see design decision 2).
  - Update the comments.
  - Verify: `actionlint` passes, and no remaining reference to tagging.
- [x] 2.5 Delete `scripts/next-release-tag.sh`. Verify: `grep -r next-release-tag` returns nothing.

## 3. Web UI

- [x] 3.1 `web/src/api/types.ts`: extend `AppVersion` with `channel: 'release' | 'edge' | 'dev'` and `commit: { sha; subject; date } | null`. Add an `aVersion` builder to `web/src/test/helpers.tsx`. Verify: `npm run typecheck` passes.
- [x] 3.2 Add `web/src/components/ui/popover.tsx`: shadcn-style wrapper over radix `Popover`, same style as `tooltip.tsx`. Verify: typecheck passes.
- [x] 3.3 `VersionLabel.tsx`:
  - For `edge` builds, render a `Badge` reading `edge` that opens a popover. Show the short-sha link to `https://github.com/sergigp/yarrtube/commit/<sha>` (new tab), the subject, and `formatDateTime(date)`.
  - Keep the muted text for `release` and `dev` builds.
  - Verify: `AppShell.test.tsx` covers:
    - release text shown
    - edge badge shown with no version text
    - clicking the badge opens the popover with the exact link href, sha text, subject and date
    - Escape closes it
    - clicking the badge stays on the current route
  - Existing loading and error tests still pass with `aVersion` fixtures.
- [x] 3.4 Verify: `npm run check` passes.

## 4. Docs

- [x] 4.1 `doc/DEVELOPMENT.md`: add a `## Deploy` section covering:
  - every passing merge to `main` publishes `:edge` + `:sha-<short>`; the NAS pulls `:edge` every 5 minutes
  - rollback by pinning `:sha-<short>`
  - manual workflow runs publish only `:sha-<short>`, for testing a branch
  - cutting a stable release: once `:edge` has been tested, `gh release create vX.Y.Z --target <sha> --generate-notes` (or push a `vX.Y.Z` tag) → `:latest`, `:X.Y.Z`, `:X.Y`
  - the tag is the version; `Cargo.toml` needs no bump

  Verify: the section exists, and its commands match the workflows.
- [x] 4.2 `doc/INSTALLATION.md`: document the image tags (`latest` recommended, `X.Y.Z`/`X.Y` for pinning, `edge` = unreleased `main`) near "Run the container" / "Updating". Verify: the tags listed match `release.yml`.
- [x] 4.3 `CLAUDE.md`: rewrite the release paragraph (edge on merge, manual stable tags, the tag is the version) and point to the Deploy section. Verify: no mention of automatic tagging is left.

## 5. Full verification

- [x] 5.1 Verify: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `cargo test --locked` pass.
