## Why

Channels and playlists are the only way to organise the library, but users
consume by topic (chess, sports, science), across many sources. Categories
are the user's own way to group sources and watch a topic as one feed. They
are also the foundation for later work: per-video categories, auto-tagging
from transcripts, personalised home rows and media-server export.

## What Changes

- **Categories**: user-created, identified by a name (no colour, icon or
  manual ordering yet). A category has members; in this change members are
  tracked channels and playlists, N-N (a source can be in many categories).
  Membership is modelled per kind so that individual videos can become
  members in a later change without reshaping the feature.
- **Unwatched count** of a category: the number of distinct downloaded,
  unwatched YouTube videos across all its members (a video tracked by two
  members counts once).
- **Sidebar Categories section**, above Channels:
  - heading with a manage control (gear) opening the manage page;
  - "+ Add category" under the heading, opening a dialog asking only for a
    name; on create, the user lands on the manage page with the new
    category open and its add-members picker focused;
  - every category always listed (never trimmed, no search), each with its
    unwatched badge, ordered like Channels: unwatched first, most unwatched
    first, ties and caught-up categories by name ignoring case;
  - row menu: "Manage" (opens it on the manage page) and "Delete"
    (confirmed);
  - selecting a category opens its browse page.
- **Manage page** (`/categories`), also reachable from the header Settings
  menu ("Categories"): master-detail on desktop (category list with member
  counts, selected category on the right), stacked list-then-detail on
  mobile. For the selected category: rename in place, an "In <name>" list
  grouped by kind (Channels, Playlists) with Remove, a searchable checkbox
  list of non-member sources to add in bulk, and Delete with a confirmation
  that states the member count and that no videos are deleted. A
  "+ New category" control creates categories from the page.
- **Browse page**: the category's videos across all members, each YouTube
  video shown once, newest first, using the home-style cards, with "Mark all
  watched" and "Manage". An empty category invites the user to add members.
- **Mark category watched**: marks every downloaded video of every member
  watched (every stored copy, as for channels).
- **Add/Edit channel and playlist dialogs** gain a Categories chip picker to
  assign the source to existing categories, or create one inline.
- Deleting a channel or playlist removes it from every category; deleting a
  category never deletes sources or videos.

Out of scope: Plex/media-server export, home personalisation, Ollama
auto-categorisation, video members, category colours/icons.

## Capabilities

### New Capabilities

- `category-crud`: create, rename, list and delete categories; add and
  remove channel and playlist members; cleanup when a source is deleted;
  per-category unwatched count.

### Modified Capabilities

- `video-listing`: list the videos of a category across its members, each
  YouTube video once, newest first.
- `watch-state`: mark a category watched.
- `web-ui`: sidebar Categories section and ordering, Settings menu entry,
  manage page, browse page, Categories field in the add/edit channel and
  playlist dialogs.

## Impact

- Backend: new category domain and SQLite tables (category, membership) via
  a migration; new HTTP endpoints for categories, membership, category
  videos and mark-watched; channel/playlist deletion cleans memberships.
- Web UI: new routes and views (manage, browse), sidebar section, dialog
  field, API types and fixture builders (`aCategory`) kept in step with the
  Rust DTOs.
- Depends on `foldable-sidebar-sections` (the Categories heading is
  foldable like the others). Playlist unwatched badges ship separately; the
  category count already includes playlist videos.
