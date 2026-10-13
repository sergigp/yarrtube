## Why

A channel's avatar is downloaded only once, when the channel is created. Images before 0.8.1 stored avatars inside the container, so recreating it lost them. Channels still record the old filenames, but the files are gone. Since 0.9.0, a channel's avatar is also its Plex show `poster.jpg`, so every affected channel now has no poster, and nothing ever downloads the avatar again.

## What Changes

- Every channel reconcile pass, before writing the channel's show files, checks whether the channel's avatar file is present. If the channel has no recorded avatar, or its recorded file is missing, the pass looks the channel up on YouTube again, downloads and stores its avatar, and records the filename on the channel.
- Best effort: a failed lookup, download, store or save is logged, never fails the pass, and is retried on the next pass.
- The show files written in the same pass use the restored avatar, so `poster.jpg` comes back on the first pass after upgrading.

## Capabilities

### New Capabilities

### Modified Capabilities
- `channel-avatars`: avatars are no longer resolved only at creation; a missing one is restored on reconcile.
- `channel-crud`: the recorded avatar filename is also set when a reconcile pass restores the avatar, not only at creation.

## Impact

- Domain: new channel avatar restore step in the channel reconcile pass; `Channel` gains an avatar filename transition.
- Infrastructure: the channel avatar port gains an existence check.
- YouTube Data API: one extra `channels.list` call per reconcile pass for each channel whose avatar is missing (including channels YouTube reports no avatar for).
- No configuration, HTTP API or migration changes.
