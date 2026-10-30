# Proposal

## Why

Home only offers time-based entry points (continue watching, quick watches, latest). Categories are how the user actually groups what they want to watch, so home should surface each category's unwatched backlog directly, more prominently than quick watches.

## What Changes

- The home endpoint returns a new `categories` list: up to 3 "More about <name>" sections, each with the category's id, name, color and up to 6 videos.
- Category sections sit between "Continue watching" and "Quick watches"; section order becomes Continue watching → category sections → Quick watches → Latest videos. A video still appears in at most one section; Quick watches and Latest lose the videos the category sections show.
- A category section holds the category's downloaded, unwatched videos from the home pool (playlists excluded from home never feed it), not shown above, newest sync first.
- Categories are ranked by their number of pending (downloaded, unwatched, not under Continue watching) videos, most first, ties by name ignoring case; a category left with no videos after removing those shown above is skipped and the next one takes its place.
- The web home view renders one section per returned category, headed "More about <name>", the heading linking to the category view, plus a "See all" link to the same view.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `video-listing`: "List Home Videos Across Sources" gains the category sections, their selection, ranking and the new section order.
- `web-ui`: "Home Page Layout" gains the "More about <name>" sections, their position and their links to the category view.

## Impact

- Backend: `VideoSearcher::list_home`, `HomeVideos`, `HomeLimits`, home DTO in `src/application/http/videos/dto.rs`.
- API: `GET /api/videos/home` response gains `categories` (additive, non-breaking).
- Web: `web/src/api/types.ts` (`HomeVideos`), fixture builders in `src/test/helpers.tsx`, `Home.tsx` and its tests.
