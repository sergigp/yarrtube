# Design

## Files

- `src/application/http/videos/dto.rs`: remove `description` from `VideoResponse` and `CategoryVideoResponse`.
- `src/application/http/videos/mod.rs`: drop the field from test expectations; add the tests below.
- `web/src/api/types.ts`: remove `description` from `Video`, mirroring the DTO.
- `web/src/test/helpers.tsx`: remove `description` from `aVideo`.
- `web/src/components/VideoDetail.tsx`: remove `Description`, `URL_PATTERN`, `DESKTOP_QUERY` and the More/Less toggle; render status and location always.
- `web/src/components/VideoDetail.test.tsx`: replace the description and expand tests.
- `e2e-tests/tests/mobileWatch.spec.js`: replace the "start collapsed" test.

## Types & Signatures

```rust
pub struct VideoResponse {
    // ...
    pub published_at: Option<DateTime<Utc>>,
    pub channel_name: Option<String>,
    // ...
}

pub struct CategoryVideoResponse {
    // ...
    pub published_at: Option<DateTime<Utc>>,
    pub channel_name: Option<String>,
    // ...
}
```

```ts
interface Video {
  // ...
  published_at: string | null
  channel_name: string | null
  // ...
}
```

`metadata.plot` stays: the NFO still renders it.

## Call Stack

Video listing:

```
GET videos handler
  -> VideoSearcher (unchanged; still loads VideoView.metadata)
  -> VideoResponse::from(view)  // reads metadata.published_at, metadata.studio
```

Detail pane:

```
VideoDetail({ basePath, video, channel, playbackRate, onPlaybackRateChange })
  -> title row, MetaLine
  -> status Badge when video.status !== 'DOWNLOADED'
  -> storage location
```

## Test Plan

1. Behaviour tests
   - Existing `it_should_include_the_metadata_when_listing_playlist_videos`, its channel and category counterparts and the absent-metadata tests: whole-value assertions, updated for the removed field. No new Rust test: a removed field can't fail one, and raw-JSON DTO tests are against the conventions.
   - VideoDetail `shows the storage location with no More control`: location visible on first render, no "More" button, at the non-desktop viewport the stub reports.
   - VideoDetail `shows the status of a video that is not downloaded`: status visible on first render.
2. Infrastructure tests

   None: no adapter changes. The existing `<plot>` test keeps the NFO covered.
