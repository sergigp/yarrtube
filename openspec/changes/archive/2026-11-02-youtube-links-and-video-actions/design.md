# Design

## Context

Web UI only; see proposal.md. The code and tests are already on branch `go-to-youtube-buttons` (uncommitted), so this design records them. The desktop breakpoint is Tailwind's `md` (48rem), the same one `VideoDetail`'s `DESKTOP_QUERY` uses.

## Goals / Non-Goals

**Goals:** source actions stay in the page header; the video's actions live in its detail pane's title row.

**Non-Goals:** marking a video unwatched by hand (it still happens only by rewatching past 10%); a custom player, or moving the speed control into it.

## Decisions

- Mobile "Mark watched" is just the tick (the label is hidden with `max-md:hidden`, and `aria-label` keeps its name). With the label, the title would get only about 55px on a 375px phone.
- The source link takes its label from the caller (`youtubeLink.label`), so `DetailHeader` stays generic. Category views pass none.
- "Synced" is hidden together with its separator on mobile so the meta line fits on one line. A line made up only of "Synced" would render empty on mobile, which is accepted.

## Files

- `web/src/lib/sourcePath.ts`: `youtubeChannelUrl` / `youtubePlaylistUrl`, next to the in-app source paths.
- `web/src/components/DetailHeader.tsx`: `HeaderAction.href` (renders a link, as an icon button or menu item); `youtubeLink` prop.
- `web/src/components/ChannelDetail.tsx`, `PlaylistDetail.tsx`: pass `youtubeLink` with the source-specific label.
- `web/src/components/MarkWatchedButton.tsx` (renamed from `VideoWatchedControl.tsx`): button only, null when watched or not downloaded.
- `web/src/components/VideoDetail.tsx`: title-row action cluster, `YouTubeLink`, meta line with "Watched" first and "Synced" hidden on mobile, link removed from the "More" section.
- `web/src/components/VideoListPane.tsx`: ⋮ on every row, placeholder removed.
- Tests next to each file; `e2e-tests/tests/channel.spec.js`, `e2e-tests/tests/mobileWatch.spec.js`.

## Types & Signatures

```ts
// sourcePath.ts
export function youtubeChannelUrl(id: string): string   // https://www.youtube.com/@handle
export function youtubePlaylistUrl(id: string): string  // https://www.youtube.com/playlist?list=<id>

// DetailHeader.tsx
export interface HeaderAction {
  label: string
  icon: ReactNode
  onSelect?: () => void   // ignored when href is set
  href?: string           // link opening in a new tab
  disabled?: boolean
  menuOnDesktop?: boolean
}
interface DetailHeaderProps { /* … */ youtubeLink?: { href: string; label: string } }

// MarkWatchedButton.tsx
interface MarkWatchedButtonProps {
  videoId: string; title: string; watched: boolean; downloaded: boolean; className?: string
}
export function MarkWatchedButton(props: MarkWatchedButtonProps): JSX.Element | null

// VideoDetail.tsx (module-private)
function YouTubeLink({ videoId }: { videoId: string }): JSX.Element
```

## Call Stack

- Source link: `ChannelDetail` → `DetailHeader({ youtubeLink: { href: youtubeChannelUrl(id), label: 'Open channel on YouTube' } })` → `actions.push({ label, icon, href })`. On desktop this renders through `IconAction` (`<Button asChild><a target="_blank">`); below it, through the `MoreActionsMenu` item (`<DropdownMenuItem asChild><a>`). `PlaylistDetail` does the same with `youtubePlaylistUrl(id)`.
- Mark watched: `VideoDetail` → `MarkWatchedButton({ videoId, title, watched, downloaded })` → click → `useMarkVideoWatched()(videoId)` → `POST /api/videos/{id}/watched` → the queries refetch → `video.watched` becomes true → the button unmounts and `MetaLine` shows "Watched".
- Video link: `VideoDetail` → `YouTubeLink({ videoId })` → `<a href="https://www.youtube.com/watch?v=<id>" target="_blank">`.

## Test Plan

1. Behaviour (Vitest, jsdom):
   - `sourcePath`: builds the channel URL (keeps `@`, encodes non-Latin) and the playlist URL.
   - `DetailHeader`: on mobile, the ⋮ menu item links the source in a new tab; on desktop, an icon link does.
   - `ChannelDetail` / `PlaylistDetail`: menu order includes "Open channel/playlist on YouTube"; the desktop icon link points to the playlist URL; every row, the selected one included, has "Actions for …".
   - `MarkWatchedButton`: marks the video and refreshes the library; renders nothing when watched; renders nothing when not downloaded; alerts and stays enabled when the request fails; is disabled while the request is in flight.
   - `VideoDetail`: the title-row link opens the video in a new tab while the details are collapsed; expanding shows the description and location (no link); a watched video's meta line reads `Watched · Published …` with no button; marks the video from the pane.
2. End-to-end:
   - `channel.spec.js`: mark watched from the pane; watched shows on the meta line; a rewatch brings the button back.
   - `mobileWatch.spec.js`: the YouTube link is visible while collapsed; the location is hidden until "More" is tapped.
