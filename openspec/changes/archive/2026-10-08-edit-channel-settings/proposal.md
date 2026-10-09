## Why

A channel's video quality and video limit can only be chosen when the channel is added (issue #88). Changing either today means deleting the channel, which deletes its downloaded videos, and adding it again.

## What Changes

- New `PATCH /channels/{handle}` endpoint that changes a channel's `quality` and/or `video_limit`. Both fields are optional, at least one is required, and each is validated as on creation. An absent field keeps its stored value. No domain event is published.
- The changed settings take effect from the next sync: the new quality applies to downloads queued after the change (already-downloaded videos and already-queued downloads keep theirs); the new limit applies at the next reconcile, so raising it downloads more of the channel's recent uploads and lowering it removes the oldest videos beyond it, deleting their files, per the existing `channel-video-sync` behaviour.
- `GET /channels` entries gain `quality` and `video_limit`, so the UI can prefill the edit form.
- Web: the channel page header "⋮" menu gains an "Edit settings" item that opens an "Edit channel" dialog with the video quality select and video limit field, prefilled with the current values. Lowering the limit shows a warning that the oldest downloaded videos beyond the new limit will be deleted. Saving a changed limit also triggers a sync of the channel so the change applies straight away.
- Out of scope: changing a channel's storage path (needs moving files on disk, as the maintainer noted on the issue) and playlist settings.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `channel-crud`: new requirement to update a channel's quality and video limit; listing returns quality and video limit.
- `web-ui`: channel page header "⋮" menu gains "Edit settings"; new edit channel dialog requirement.

## Impact

- **Domain**: `Channel` gains `with_quality` / `with_video_limit` transitions; new `UpdateChannelError`; new `ChannelUpdater` use case; `ChannelView` gains `quality` and `video_limit`.
- **Infrastructure**: `ChannelRepository::update` port method + SQLite implementation. No migration (columns already exist).
- **HTTP**: new `UpdateChannelRequest` DTO, `update_channel` handler and `PATCH /channels/{handle}` route; `ChannelListItemResponse` gains two fields.
- **Web**: `api/types.ts` + `aChannel` builder, `updateChannel` client function + query hook, new `EditChannelDialog`, `EntryActionsMenu` / `DetailHeader` / `ChannelDetail`.
- No breaking API change: the list response only gains fields.
