# Proposal

## Why

Videos' chapters are now stored and listed (`video-chapters`), but the web
player ignores them. Chapters are the best way to navigate long videos
(matches, streams, podcasts), and a thin marked seek bar (YouTube's
approach) hides them. They deserve a dedicated, readable place in the UI.

## What Changes

- A chapter rail directly under the player in the playlist, channel and
  category players: one fixed-width card per chapter showing its start
  time and title, scrolling sideways. Text only, no images.
- The chapter playing now is highlighted, with a fill showing progress
  through it, and the rail keeps it in view as playback moves on.
- Clicking a card seeks to that chapter's start and plays.
- Shown only for a downloaded video with chapters; otherwise nothing is
  shown in its place.
- On desktop the player's maximum height shrinks a little to make room.
- On mobile the rail stays pinned under the player with it, and ends
  with a button that opens the full chapter list in a sheet; picking a
  chapter there seeks to it and closes the sheet.
- Out of scope (first iteration, revisit after trying it): chapter
  keyboard shortcuts, chapter frame images, the full-list button on
  desktop, chapter counts in video lists, timestamp links in
  descriptions. No backend changes.

## Capabilities

### New Capabilities
- `player-chapters`: how the web player presents a video's chapters and
  lets the user navigate them (rail, current chapter, seeking, mobile
  chapter list).

### Modified Capabilities

## Impact

- Web only: `VideoPlayer` (shared by the playlist, channel and category
  players) gains the rail; new chapter rail and chapter list components,
  a current-playback-time hook and pure chapter logic in `web/src/lib/`.
- No API, backend or database changes: listings already return
  `chapters`.
