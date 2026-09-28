## Files

Backend:
- `src/domain/video/recent_video.rs`: `VideoSource` variants gain the source name and become struct variants. Two positional `String`s would be easy to swap by mistake.
- `src/domain/services/video_searcher.rs`: `recent_from_playlists` / `recent_from_channels` fill in `name` from the `Playlist` / `Channel` they already load.
- `src/application/http/videos/dto.rs`: `RecentVideoSourceResponse` gains `name`, mapped from `VideoSource`.
- `src/application/http/videos/mod.rs`: handler tests. The `playlist_source` / `channel_source` helpers gain the name, plus the new name tests.

Frontend:
- `web/index.html`: favicon, `apple-touch-icon`, `manifest` and `theme-color` links.
- `web/public/favicon.svg`: replaced with the Yarrtube mark (a `#2e7d4f` rounded square with a white bold "Y").
- `web/public/apple-touch-icon.png`: 180×180 PNG rendered from `favicon.svg`.
- `web/public/icon-192.png`, `web/public/icon-512.png`: manifest icons rendered from `favicon.svg`.
- `web/public/manifest.webmanifest`: `name`, `short_name`, `start_url: "/"`, `display: "standalone"`, `theme_color`, `background_color`, icons.
- `web/src/App.jsx`: the shell scrolls the document on mobile. The header is sticky with a fixed mobile height. The desktop keeps the `h-dvh` shell with inner scrolling (`md:` classes).
- `web/src/components/Sidebar.jsx`: a `...` `DropdownMenu` per row replaces the hover buttons. The drawer is `w-64`. The page behind it is locked while it's open.
- `web/src/components/Home.jsx`: the card splits into a video link (thumbnail and title) and a channel link (avatar and `source.name`).
- `web/src/components/VideoPlayer.jsx` (new): player area shared by both detail views. On mobile it's `aspect-video`, sticky below the header, with no `min-h-80`.
- `web/src/components/VideoDetail.jsx` (new): detail pane shared by both detail views. The title has its own row, and the rest is collapsible.
- `web/src/components/DetailHeader.jsx` (new): page header shared by both detail views. It shows the name, avatar (channels), a video/unwatched summary, and Sync / Mark all watched / Delete actions. It owns the pending states and the delete `ConfirmDialog`. `VideoPlayer` drops its mobile `-mt-4` so it sits one gutter below this header.
- `web/src/components/PlaylistDetail.jsx`, `web/src/components/ChannelDetail.jsx`: render `DetailHeader` above the grid, and use the shared player and detail. Their layout becomes a single document-flow column on mobile, and the desktop grid is unchanged.

The two detail views are currently copies of each other. Extracting `VideoPlayer` and `VideoDetail` means each fix here is made once instead of twice.

## Types & Signatures

```rust
// src/domain/video/recent_video.rs
pub enum VideoSource {
    Playlist {
        id: PlaylistId,
        name: PlaylistName,
        path: PlaylistPath,
    },
    Channel {
        handle: ChannelHandle,
        name: String,
        path: PlaylistPath,
        avatar_filename: Option<String>,
    },
}

// src/application/http/videos/dto.rs
pub struct RecentVideoSourceResponse {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub path: String,
    pub avatar_filename: Option<String>,
}
```

```jsx
// web/src/components/VideoPlayer.jsx
export function VideoPlayer({ basePath, video, autoplay, onVideoElement })

// web/src/components/VideoDetail.jsx
// channel is optional: when given, renders its avatar as a link to /channels/:id
export function VideoDetail({ basePath, video, channel })

// web/src/components/DetailHeader.jsx
// onMarkWatched is optional (channels only); onDelete resolves after deleting and navigating home
export function DetailHeader({ name, avatarSrc, showAvatar, videos, unwatchedCount, onSync, onMarkWatched, onDelete, deleteDescription })

// web/src/components/Sidebar.jsx
function SidebarRowMenu({ item, onSync, onMarkWatched, onDeleteRequest })
```

```css
/* web/src/index.css: header height shared by the sticky header and sticky player */
:root { --header-height: 4rem; }
```

## Call Stack

Recent videos (API):
```
GET /videos/recent?limit=N
  -> http::videos::list_recent_videos(State<VideoSearcher>, Query<ListRecentVideosQuery>)
     -> VideoSearcher::list_recent(limit)
        -> recent_from_playlists()  -> VideoSource::Playlist { id, name: playlist.name, path }
        -> recent_from_channels()   -> VideoSource::Channel { handle, name: channel.name, path, avatar_filename }
     -> RecentVideoResponse::from(RecentVideo) -> RecentVideoSourceResponse { kind, id, name, path, avatar_filename }
```

Home card:
```
Home -> usePolling(fetchRecentVideos)
  -> VideoGrid -> <li>
       <Link to=/{kind}s/{id}?video={videoId}>  thumbnail + title
       <Link to=/channels/{source.id}>          avatar + source.name   (kind === 'channel')
```

Detail view (mobile):
```
App (document scrolls; header sticky top-0, h-[--header-height])
  -> PlaylistDetail | ChannelDetail
       -> DetailHeader { name, avatar?, summary, Sync / Mark all watched? / Delete -> ConfirmDialog -> navigate('/') }  scrolls with the page
       -> VideoPlayer { basePath, video, autoplay, onVideoElement }  sticky top-[--header-height], aspect-video
       -> VideoDetail { basePath, video, channel? }
            title (own row) + [more] toggle
              -> useState(() => matchMedia('(min-width: 768px)').matches)   // expanded by default on desktop
            collapsible: badges, path, "Open on YouTube"
       -> video list (document flow)
```

Sidebar row:
```
SidebarRow -> Link (avatar, name, unwatched badge)
           -> SidebarRowMenu -> DropdownMenu
                -> Sync            -> onSync()
                -> Mark all watched -> onMarkWatched()   (channels only)
                -> Delete          -> onDeleteRequest() -> ConfirmDialog
Sidebar(open) -> useEffect: document.body.style.overflow = open ? 'hidden' : ''
```

## Test Plan

1. Behaviour tests (`src/application/http/videos/mod.rs`, against real SQLite):
   - `it_should_include_channel_name_in_recent_videos`: a channel "Some Channel" with a downloaded video, so the response's source has `name: "Some Channel"`.
   - `it_should_include_playlist_name_in_recent_videos`: a playlist "My Playlist" with a downloaded video, so the response's source has `name: "My Playlist"`.
2. Infrastructure tests: none. No adapter or repository changes.

`web/` has no automated test harness. The UI is verified manually with `scripts/run-local.sh`:
- in a desktop browser, checking that the desktop layout hasn't regressed
- on a real iPhone in Safari, for the bars and page scrolling
- from the iPhone home screen, for standalone mode
