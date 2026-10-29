# Proposal

## Why

Categories are browse-only: opening a video from a category view jumps to its channel or playlist, so the player queues that source instead of the category. Two smaller polish items come along. Home card tints are too strong. The add/edit dialogs hide the few existing categories behind a type-to-search field, although picking an existing category is the common case and there are rarely more than 10.

## What Changes

- **Category playback**: a card on a category view opens the video within the category (`/categories/:id?video=<id>`). The view shows the player and the video's detail, with the category's videos as the queue beside them, like a playlist view. Each video still plays from, and reports progress against, its own channel or playlist.
- **Category videos endpoint returns full videos**: `GET /api/categories/:id/videos` returns every field a playlist or channel video listing returns (status, filename, quality, timestamps, sync time, metadata, subtitle language), plus the existing `source` and `category_ids`. This is additive: every field it returned before is kept.
- **Categories field as chips**: the add/edit channel and playlist dialogs show every category as a toggle chip, ordered by name. A chosen chip is filled with its category's color; the others are outlined with a dot in their color. A "New category" field creates a category and chooses it, or chooses the existing one when its name is typed. The type-to-search combobox is removed.
- **Softer home card tints**: category tints on home cards are drawn at 50% opacity, including the neutral grey for 4 or more categories.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `video-listing`: "List Videos For A Category" returns the full playlist/channel video fields as well as the source and categories.
- `web-ui`: "Category View" cards open the video in the category, plus a new "Category Playback" requirement; "Categories Field In Add And Edit Dialogs" changes to toggle chips in category colors with a create field; "Home Card Category Tint" tints are half transparent.

## Impact

- Backend: `src/domain/services/video_searcher.rs` (`list_for_category` builds a richer view), a new domain view in `src/domain/video/`, `src/application/http/videos/{mod.rs,dto.rs}` (new response DTO and handler return type, with the existing category tests updated).
- Frontend: `web/src/api/{types,client,queries}.ts`, `CategoryDetail`, a new `CategoryPlayer`, `VideoListPane`, `VideoGrid`, `useVideoSelection`, `CategoryPicker`, `lib/categoryColors.ts`, plus their tests.
- E2E: `e2e-tests/tests/categories.spec.js`, for the picker interaction.
- API: additive fields on the category videos response; no consumers other than the embedded web UI.
