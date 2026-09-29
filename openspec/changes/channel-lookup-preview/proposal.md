## Why

The add playlist dialog now looks the playlist up on YouTube as it is typed, so a wrong ID, an unknown playlist or one already tracked is reported before submission. The add channel dialog still accepts any handle and only finds out that it doesn't exist, or that it is already tracked, after the request is sent. The backend already resolves a handle's title and avatar at creation, so the dialog can confirm the channel, and show whose it is, before anything is created.

## What Changes

- New `GET /api/channels/preview?channel=<handle or URL>` returns the channel's handle, YouTube title and avatar URL without persisting anything, publishing an event, storing an avatar or scheduling a task. It rejects a value that isn't a handle or channel URL, reports a channel YouTube doesn't know as not found, and a failed lookup as a bad gateway.
- The **add channel destination notice** is fed by that lookup, in the same order of cases as the playlist notice:
  - "Looking up channel…" while the field is still changing or the lookup is in flight
  - an error when the value is not a handle or channel URL, when the channel is not found, or when YouTube can't be reached
  - an error "Already added as “<name>”" when the channel is already tracked
  - the existing destination-in-use error
  - otherwise the channel's avatar, then "The latest N videos from “<title>” will be downloaded to <path>" and the "change" action
- The dialog does not submit until the channel has been found, and not while it is already tracked.
- Unchanged: the folder name still derives from the handle, not the title, and the notice still states the video limit rather than a video count.
- The avatar shown is the URL YouTube returns. Nothing is stored until the channel is created, when the existing avatar storage takes over.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `channel-crud`: a new requirement adds the channel preview endpoint. Create Channel is unchanged.
- `web-ui`: "Add Channel Destination Notice" gains the lookup, the looking-up, error and already-added states, the channel's title and avatar, and blocks submission until the channel is found.

## Impact

- **Backend**:
  - New `ChannelPreviewer` domain service over the existing `YoutubeChannelRepository::resolve`, and a `ChannelPreview` read model with a `PreviewChannelError`.
  - The channels HTTP module gains a `preview_channel` handler, and `ApiServices`/`serve.rs` wire the previewer.
  - The route `GET /channels/preview` is registered ahead of `/channels/{handle}`.
  - `YoutubeApiChannelRepository` treats a YouTube response with no `items` field as "no such channel". YouTube leaves the field out when a handle matches nothing, so today an unknown channel fails to parse and shows up as a bad gateway, in Create Channel as well as in the new preview.
- **Frontend**:
  - `web/src/api.js` and `web/src/queries.js` gain the channel preview call.
  - `web/src/channelNotice.js` names the channel's title.
  - `web/src/components/AddChannelDialog.jsx` runs the debounced lookup and renders the avatar in the notice.
- **Smoke tests**:
  - `addChannelDialog.spec.js` gains the lookup cases.
  - Its existing cases, and the channel cases in `addDialogLocation.spec.js`, move from the made-up `@some-handle` to the real smoke channel, since a made-up handle now shows "not found".
  - `channel.spec.js` checks the already-added state.
- **Data**: no migration.
