# Proposal

## Why

A category is today only a group of channels and playlists, and every
category read (feed, counts, home chips, mark watched) re-derives its videos
from the members on each request. Plex episode labels (`plex-category-labels`)
and, later, AI classification both need categories on the individual video
record, so the record → category relation has to exist as data before
either can be built.

## What Changes

- New `video_categories` relation between a video **record** (`videos.id`,
  one per playlist/channel copy, i.e. one Plex episode) and a category. A
  YouTube video tracked by a channel and a playlist has two records with
  independent categories.
- Categories of a record follow one of two regimes, derived from data (no
  origin column):
  - **Source regime**: the record's playlist/channel is a member of at least
    one category → its categories are exactly its source's categories.
  - **AI regime**: its source is in no category → its categories are set by
    a classifier. Nothing writes them in this change (classification is a
    future change); such records simply have no categories.
- The relation is maintained, set-based, when:
  - a category's members are set (every record of the old and new member
    sources is rewritten in the same transaction as the members; a source
    left in no category loses its rows),
  - a category is deleted (its rows go with it, same transaction),
  - a record is deleted (its rows go with it),
  - a video sync runs for a record (`video-reconciler`): its categories are
    assigned from its source when the source is in a category,
  - a playlist/channel reconcile pass runs: its records are re-assigned from
    the source, repairing any drift.
  - Renaming or recoloring a category touches no rows.
- Upgrade backfills the relation from the current memberships.
- Every category read (category feed, unwatched counts and total, home
  videos' category ids, mark category watched) reads the relation instead of
  walking members. User-visible results are unchanged.
- New `VideoCategoriesChanged` domain event carrying, in one batch, the
  records whose categories (or one of whose category names) changed, so
  `plex-category-labels` can react without polling. Nothing subscribes to it
  yet.

Out of scope: AI/ollama classification, manual per-video tagging (not
planned), any API or UI change.

Depends on `video-reconciler` (the video sync hosts the per-record
assignment). `plex-category-labels` builds on this change.

## Capabilities

### New Capabilities
- `video-categories`: which categories each stored video record has, the
  source/AI regime rule, when the relation is maintained, the upgrade
  backfill, and the change notification.

### Modified Capabilities
<!-- None: category-crud, video-listing and watch-state keep their requirements;
     their results are now computed from the relation but are identical. -->

## Impact

- Schema: migration `0014_video_categories.sql` (table, index, backfill).
- Domain: new `video_category` module (`VideoCategoriesChanged`,
  `CategorizedVideo`), new `VideoCategoryAssigner` service;
  `CategoryVideoCollector`, `CategoryUpdater`, `CategoryDeleter`,
  `VideoReconciler`, `ChannelVideoReconciler`, `PlaylistVideoReconciler`
  change; `DomainEvent` gains a variant.
- Infrastructure: new `VideoCategoryRepository` port + SQLite adapter;
  `SqliteCategoryRepository` update/delete and `SqliteVideoRepository::delete`
  maintain the relation.
- No API, DTO, web UI or environment variable change.
