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
  - "Add category": a full-width row while no category exists, a "+" on
    the heading once one does, opening a dialog asking only for a name; on
    create, the user lands on the new category's page with its manage
    sources dialog open;
  - every category always listed (never trimmed, no search), each with its
    unwatched badge, ordered like Channels: unwatched first, most unwatched
    first, ties and caught-up categories by name ignoring case;
  - row menu: "Manage sources" (opens the category page with the manage
    sources dialog) and "Delete" (confirmed);
  - selecting a category opens its page.
- **Category page**: the category's videos across all its sources, each
  YouTube video shown once, newest first, using the home-style cards. Its
  header follows the channel and playlist pages: the name with a pencil to
  rename it in place, a summary (sources, videos, unwatched) and a "⋮" menu
  with "Mark all watched", "Manage sources" and "Delete" (confirmed, stating
  the source count and that no videos are deleted). An empty category
  invites the user to add sources.
- **Manage sources dialog**: every tracked channel and playlist as a chip
  (channels with their avatar), pressed when in the category, with a search,
  All / In / Not in filters, and staged changes saved at once. There is no
  separate management page.
- **Mark category watched**: marks every downloaded video of every member
  watched (every stored copy, as for channels).
- **Add/Edit channel and playlist dialogs** gain a Categories chip picker to
  assign the source to existing categories, or create one inline.
- **Sidebar section headings** (Categories, Channels, Playlists): once a
  section has items, its add control becomes a compact "+" at the
  heading's trailing edge (the full-width row stays for an empty section);
  a folded heading's unwatched total uses the green row badge instead of
  muted text.
- Deleting a channel or playlist removes it from every category; deleting a
  category never deletes sources or videos.

Out of scope: Plex/media-server export, home personalisation, Ollama
auto-categorisation, video members, category colours/icons, marking a
whole sidebar section watched from its heading.

## Capabilities

### New Capabilities

- `category-crud`: create, rename, list and delete categories; add and
  remove channel and playlist members; cleanup when a source is deleted;
  per-category unwatched count.

### Modified Capabilities

- `video-listing`: list the videos of a category across its members, each
  YouTube video once, newest first.
- `watch-state`: mark a category watched.
- `web-ui`: sidebar Categories section and ordering, category page with
  in-place rename and the manage sources dialog, Categories field in the
  add/edit channel and playlist dialogs; sidebar section add controls
  (row when empty, heading "+" otherwise) and the folded total badge.

## Impact

- Backend: new category domain and SQLite tables (category, membership) via
  a migration; new HTTP endpoints for categories, membership, category
  videos and mark-watched; channel/playlist deletion cleans memberships.
- Web UI: a category route and view, the manage sources dialog, sidebar
  section, dialog field, API types and fixture builders (`aCategory`) kept in step with the
  Rust DTOs.
- Depends on `foldable-sidebar-sections` (the Categories heading is
  foldable like the others). Playlist unwatched badges ship separately; the
  category count already includes playlist videos.
