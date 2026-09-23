## Why

A channel's video limit is only checked for being positive, then narrowed to 32 bits with a wrapping cast. A value above `u32::MAX` wraps silently: `4294967306` is stored as `10`, and `4294967296` is stored as `0`. A stored limit of `0` then fails validation on every read, so a single create request can make listing every channel fail with a 500 and leave that channel impossible to delete or reconcile through the API. The bound needs to be explicit and enforced at the boundary before a value is persisted.

## What Changes

- A channel's video limit must be an integer from 1 to 1000 inclusive. A create request whose video limit is below 1 or above 1000 is rejected with 400 Bad Request, and nothing is persisted.
- The rejection message states the accepted range and the value received, e.g. `Video limit must be between 1 and 1000 (got 1001)`. This replaces the current `Video limit must be a positive integer (got N)` message for out-of-range values; the missing-value message is unchanged.
- The web UI's channel video limit input declares a maximum of 1000 alongside its existing minimum of 1.
- **BREAKING**: create requests with a video limit above 1000, previously accepted (and possibly stored as a wrapped, different value), are now rejected with 400.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `channel-crud`: the create-channel requirement's video limit changes from "a positive integer" to "an integer between 1 and 1000 inclusive", and the invalid-video-limit scenario covers values above 1000.
- `web-ui`: the add dialog's channel video limit field accepts values from 1 to 1000.

## Impact

- `src/domain/channel/video_limit.rs`: `VideoLimit::new` enforces the 1..=1000 range; its value-object tests cover both bounds.
- `src/application/http/channels/mod.rs`: the handler is unchanged (it already maps `ValidationError` to 400); its one invalid-value test stays representative.
- `web/src/components/AddDialog.jsx`: `max="1000"` on the video limit input.
- API: `POST /channels` rejects video limits above 1000. No new endpoints or dependencies.
- Stored data: no migration. Existing stored limits are within range.
