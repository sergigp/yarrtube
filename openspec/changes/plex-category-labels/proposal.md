# Proposal

## Why

yarrtube's categories only exist in yarrtube: in Plex, a category's videos
are scattered across many shows with no way to watch "everything new in
Chess" as one feed. A spike on Plex Media Server 1.43.4 confirmed that
episodes accept arbitrary labels via the API (N-N) and that smart playlists
can filter episodes by label, so each category can become a cross-show,
self-updating Plex feed. `categories-per-video` gives every video record its
categories; this change mirrors them into Plex.

## What Changes

- When the Plex integration is configured (the same three variables as the
  scan trigger), every downloaded video's Plex episode carries exactly its
  categories' names as labels. yarrtube owns episode labels in its shows: it
  writes the full list and removes any other label.
- Labels follow every category change (members set, rename, delete) and
  every new download. Writing them is a background **Plex label sync**:
  records whose labels may be stale are marked, and the sync applies them in
  batches. An episode Plex hasn't imported yet (its scan is asynchronous),
  or a Plex server that is down, simply stays marked and is retried by a
  later sync; downloads are never affected.
- The label sync runs in its own task lane, so it never blocks downloads,
  thumbnails or subtitles. It runs shortly after a category change or a
  download, and periodically to heal drift (an episode Plex re-imported
  with a new id, a categorized video that was never synced, a deleted
  video).
- For each category, yarrtube creates a **smart playlist** in each Plex
  library that holds labelled episodes of it: that category's unwatched
  episodes, newest air date first. Playlists are replaced when the category
  is renamed, deleted when the category has no labelled episode left in
  that library, and recreated if deleted in Plex. They belong to the Plex
  user whose token yarrtube uses.
- The tasks view describes the new label sync task in plain language (in
  the **Other** tab, like other Plex work).
- `doc/PLEX.md` documents labels, the playlists and their limits.

Out of scope: playlists for other Plex users (managed/shared users),
genres/Categories tab, AI classification, making the playlist definition
configurable.

## Capabilities

### New Capabilities
- `plex-category-labels`: mirroring each video's categories to its Plex
  episode's labels, keeping them in sync (triggers, retries, drift), and the
  per-category, per-library smart playlists built on those labels.

### Modified Capabilities
<!-- None. The tasks view requirement already covers describing Plex sync
     tasks in plain language and placing them in the Other tab. -->

## Impact

- Schema: migration `0015_plex_labels.sql` (episode label state, category
  playlists).
- Domain: new `plex_label` module, new `PlexLabelSyncer` service, new
  `Task::SyncPlexLabels` and `TaskLane::Plex`; `PlexSection` gains its title;
  new `PlexEpisode`, `PlexPlaylist`, `PlexLabels` types; the video owner
  lookup moves out of `VideoReconciler` into a shared `VideoOwnerFinder`
  (no behaviour change).
- Infrastructure: `PlexLibraryRepository` gains identity, episode listing,
  episode label read/write and playlist list/create/delete; two new SQLite
  repositories.
- Application: `SyncPlexLabelsTask`; subscribers on `VideoCategoriesChanged`
  and `VideoDownloaded`; daemon startup (`prepare_task_queue`) requests the
  periodic sync when Plex is configured.
- Web: task description for the label sync.
- No API, DTO or environment variable change. Plex writes happen only with
  the integration configured.
