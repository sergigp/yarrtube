## 1. Walking skeleton

- [x] 1.1 Create `ChannelAvatarRestorer` (struct, `new`, `ChannelAvatarRestorerApi::restore_missing` returning the channel unchanged), `Channel::with_avatar_filename`, and `ChannelAvatarRepository::exists` on the port, filesystem adapter (returning `Ok(true)`) and fake. Inject the restorer into `ChannelVideoReconciler` (called before `write_channel_show`), wire it in `serve.rs`, and pass it at every test construction site. Done when `cargo build` succeeds and `cargo test --locked` passes.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_restore_a_missing_channel_avatar`: drives re-downloading a missing recorded avatar and copying it as the poster; verify with `cargo test it_should_restore_a_missing_channel_avatar`.
- [x] 2.2 `it_should_record_a_restored_avatar_for_a_channel_without_one`: drives saving the new filename on the channel; verify with `cargo test it_should_record_a_restored_avatar_for_a_channel_without_one`.
- [x] 2.3 `it_should_keep_a_stored_channel_avatar`: drives skipping the lookup when the file is present; verify with `cargo test it_should_keep_a_stored_channel_avatar`.
- [x] 2.4 `it_should_sync_the_channel_even_if_its_avatar_cannot_be_restored`: drives best-effort failure handling; verify with `cargo test it_should_sync_the_channel_even_if_its_avatar_cannot_be_restored`.

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `FilesystemChannelAvatarRepository`, `it_should_report_a_stored_avatar_file_as_existing`: verify with `cargo test it_should_report_a_stored_avatar_file_as_existing`.
- [x] 3.2 `FilesystemChannelAvatarRepository`, `it_should_report_an_avatar_as_missing_if_its_directory_is_gone`: verify with `cargo test it_should_report_an_avatar_as_missing_if_its_directory_is_gone`.

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
- [x] 4.2 Manual check with `scripts/run-local.sh`: delete a channel's avatar file from `avatars/`, force a reconcile of that channel, and confirm the avatar file and the show's `poster.jpg` come back.
