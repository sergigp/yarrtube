Follow `/rust-architect` conventions for every Rust task, and end each task with a commit.

## 1. Directory Aggregate

- [ ] 1.1 Add `src/domain/directory/directory_path.rs` with `DirectoryPath` (`new`, `root`, `as_str`, `is_root`, `Display`), rejecting absolute paths, `..` segments, and empty segments, with the empty path meaning the videos root; verify the six `directory_path.rs` unit tests in design.md's test plan pass.
- [ ] 1.2 Add `src/domain/directory/directory.rs` with the `Directory` entity (`path`, `subdirectories`) and `src/domain/directory/errors.rs` with `DirectoryPathError` and `ListDirectoriesError`; verify `cargo build` succeeds.
- [ ] 1.3 Add `src/domain/directory/mod.rs` re-exporting the three types and register the module in `src/domain/mod.rs`; verify `cargo build` succeeds.

## 2. Filesystem Adapter

- [ ] 2.1 Add `src/infrastructure/repositories/filesystem_directory_repository.rs` with the `DirectoryRepository` port and `FilesystemDirectoryRepository::new(videos_root)`, listing one level only, keeping directories, omitting regular files and dot-prefixed names, ordered deterministically; verify `it_should_list_only_subdirectories_on_a_directory_containing_files_and_directories`, `it_should_omit_dot_prefixed_subdirectories`, and `it_should_return_entries_in_a_deterministic_order` pass.
- [ ] 2.2 Enforce confinement by canonicalizing the joined path and checking it against the canonicalized videos root, returning `Ok(None)` for a missing path, a regular file, and a path resolving outside the root alike; verify `it_should_return_none_on_a_missing_directory`, `it_should_return_none_on_a_path_that_is_a_regular_file`, `it_should_return_none_on_a_symlink_resolving_outside_the_videos_root`, and `it_should_list_the_target_on_a_symlink_resolving_inside_the_videos_root` pass.
- [ ] 2.3 Register the module in `src/infrastructure/repositories/mod.rs`; verify `cargo build` succeeds.

## 3. Directory Searcher

- [ ] 3.1 Add `src/domain/services/directory_searcher.rs` with `DirectorySearcher::list` mapping the port's `Ok(None)` to `ListDirectoriesError::NotFound` and a port failure to `ListDirectoriesError::Repository`, and register plus re-export it in `src/domain/services/mod.rs`; verify `cargo build` succeeds.

## 4. Listing Endpoint

- [ ] 4.1 Add `src/application/http/directories/dto.rs` with `ListDirectoriesQuery`, `DirectoryEntryResponse`, `DirectoryResponse` (carrying `root`, `path`, `entries`), and `DirectoryResponse::new(videos_root, directory)`; verify `cargo build` succeeds.
- [ ] 4.2 Add `src/application/http/directories/mod.rs` with `list_directories`, defaulting a missing `path` to `DirectoryPath::root()` and mapping outcomes to 200, 400 on an invalid path, 404 on not found, and 500 on a repository failure; verify the module compiles.
- [ ] 4.3 Add `directory_searcher` and `videos_root` to `AppState` and route `GET /directories` in `src/application/http/mod.rs`; verify the seven behavior tests in design.md's test plan pass against a fake `DirectoryRepository`, including `it_should_report_the_videos_root_on_every_successful_listing`.

## 5. Daemon Startup

- [ ] 5.1 Add `DEFAULT_STORAGE_DIRECTORIES` and `run_startup_storage_directories_check` to `src/serve.rs`, creating each default directory under the videos root, leaving existing ones and their contents alone, and logging and continuing on failure; verify the three `serve.rs` tests in design.md's test plan pass.
- [ ] 5.2 Call the startup check alongside the existing startup checks and wire `FilesystemDirectoryRepository` and `DirectorySearcher` into `build_application`; verify `scripts/run-local.sh` starts and `curl 'localhost:8080/api/directories'` returns `root`, `path`, and the seeded `playlists` and `channels` entries.

## 6. Add Dialog

- [ ] 6.1 Add `fetchDirectories(path)` to `web/src/api.js` calling `GET /api/directories`; verify a listing renders in the browser console against the local daemon.
- [ ] 6.2 Add `web/src/components/LocationField.jsx` with the collapsed parent display and reveal control, a breadcrumb whose every ancestor is selectable, the one-level listing, and occupied entries marked with the playlist or channel name from `fetchPlaylists`/`fetchChannels`; verify browsing from `playlists/` up to the root and back down works against the local daemon.
- [ ] 6.3 Add the create-folder step to `LocationField`, staging a parent that does not exist without any request or write, showing it as empty, and descending into an existing directory when the given name matches one; verify adopting `kids` at the root stages it and adopting an existing `playlists` descends instead.
- [ ] 6.4 Add the folder name input and destination preview to `LocationField`, rejecting `/` and an empty name, naming every directory in the destination that will be created, and blocking submission when the destination is occupied; verify `/videos/kids/$name` names both new directories and an occupied destination blocks the submit button.
- [ ] 6.5 Use `LocationField` in `web/src/components/AddDialog.jsx` for both modes, submitting the composed relative path, removing the free-text path field and the conflict auto-expand, and leaving quality and video limit in "Advanced options"; verify creating a playlist and a channel through the dialog stores the expected path.

## 7. Smoke Tests

- [ ] 7.1 Update `smoke-tests/helpers/addDialog.js` to drive the parent browser and folder name instead of the path field; verify the existing playlist and channel specs pass unchanged otherwise.
- [ ] 7.2 Add the six `smoke-tests/` cases from design.md's test plan, covering a browsed parent, a staged parent, breadcrumb navigation to a sibling, the create-folder step selecting an existing directory, a blocked occupied destination, and a rejected folder name containing a slash; verify the suite passes.

## 8. Verification

- [ ] 8.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`; verify all three succeed with no warnings.
- [ ] 8.2 Run the daemon against a videos root holding directories yarrtube did not create and walk both cases from the design's call stack end to end; verify videos download into the previewed destination and no unintended directory appears under the videos root.
