# release-channels Specification

## Purpose

Defines which container image tags are published for each kind of build: a merge to `main` (edge), a maintainer-pushed release tag (stable), and a manual test run. Users on `:latest` only receive deliberately cut releases, while the maintainer's deployment follows `main` continuously.

## Requirements

### Requirement: Merges To Main Publish Edge
Every push to `main` that passes CI SHALL publish a multi-arch image tagged `:edge` and `:sha-<short commit sha>`. It SHALL NOT create a git tag, and it SHALL NOT publish `:latest` or any semver image tag.

#### Scenario: Merge passes CI
- **WHEN** commit `abc1234` is merged to `main` and CI passes
- **THEN** `:edge` and `:sha-abc1234` point to an image built from `abc1234`, no new git tag exists, and `:latest` still points to the image it pointed to before

#### Scenario: Merge fails CI
- **WHEN** a merge to `main` fails CI
- **THEN** no image is published and `:edge` is unchanged

### Requirement: Edge Never Moves Backwards
When two merges to `main` are being published at the same time, `:edge` SHALL end up pointing to the image of the more recent merge.

#### Scenario: Two quick merges
- **WHEN** commit `bbb2222` is merged while the edge publish for the earlier commit `aaa1111` is still running
- **THEN** once publishing settles, `:edge` points to the image built from `bbb2222`

### Requirement: Release Tags Publish Stable
Pushing a git tag `vX.Y.Z` SHALL publish a multi-arch image built from the tagged commit, tagged `:latest`, `:X.Y.Z` and `:X.Y`. It SHALL NOT publish `:edge`.

#### Scenario: Maintainer pushes a release tag
- **WHEN** the maintainer pushes tag `v0.10.0`
- **THEN** `:latest`, `:0.10.0` and `:0.10` point to the image built from the tagged commit, and `:edge` is unchanged

### Requirement: Manual Runs Publish Only A Commit Image
A manually triggered release run SHALL publish only `:sha-<short commit sha>` for the selected ref. It SHALL NOT move `:edge`, `:latest` or any semver image tag.

#### Scenario: Testing a branch image
- **WHEN** the release workflow is run manually on a branch at commit `ccc3333`
- **THEN** `:sha-ccc3333` points to an image built from `ccc3333`, and `:edge` and `:latest` are unchanged
