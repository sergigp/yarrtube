## Purpose

Keeps one Plex collection per tracked playlist and channel in sync with
yarrtube's downloaded videos, so a Plex library of loose videos becomes one
browsable tile per playlist/channel with correct ordering.

## ADDED Requirements

### Requirement: Plex integration is optional and off by default

The system SHALL enable the Plex collections integration only when both
`YARRTUBE_PLEX_URL` and `YARRTUBE_PLEX_TOKEN` environment variables are set.
When the integration is disabled the system MUST NOT contact any Plex server
and MUST behave exactly as it does today.

#### Scenario: Integration disabled

- **WHEN** the daemon starts without `YARRTUBE_PLEX_URL` or without
  `YARRTUBE_PLEX_TOKEN`
- **THEN** no Plex sync task is scheduled and no requests are made to a Plex
  server

#### Scenario: Integration enabled

- **WHEN** the daemon starts with both `YARRTUBE_PLEX_URL` and
  `YARRTUBE_PLEX_TOKEN` set
- **THEN** a recurring global Plex collections sync task is scheduled

### Requirement: One collection per tracked playlist and channel

For every tracked playlist and channel, a sync pass SHALL ensure a Plex
collection named after the playlist/channel exists in each configured
library section holding videos of that playlist/channel that Plex has
scanned, creating it when missing. The system SHALL support multiple
configured library sections, reconciling each independently, so
yarrtube-fed content split across several Plex libraries is covered.
Collections created by the system SHALL be configured to sort
alphabetically, so that yarrtube's position-prefixed `sorttitle` values
order members by playlist position / publish date.

#### Scenario: Collection created for a playlist with scanned videos

- **WHEN** a sync pass runs and a tracked playlist has downloaded videos
  that Plex has scanned, and no collection with the playlist's name exists
  in the section
- **THEN** a collection with the playlist's name is created containing those
  videos, with alphabetical sorting configured

#### Scenario: Content split across libraries

- **WHEN** two library sections are configured and a playlist's scanned
  videos live in the second one
- **THEN** the playlist's collection is created in the second section, and
  no collection for it is created in the first

#### Scenario: No empty collection for content Plex has not scanned

- **WHEN** a sync pass runs and none of a playlist's downloaded videos are
  scanned by Plex yet
- **THEN** no collection is created for that playlist in this pass

### Requirement: Videos are matched to Plex items by YouTube ID

A sync pass SHALL match yarrtube videos to Plex library items by YouTube
video ID, using the `youtube://<video-id>` GUID that Plex's NFO agent
derives from the `<uniqueid type="youtube">` element yarrtube writes to
`movie.nfo`. Matching MUST NOT depend on file paths, since Plex and yarrtube
generally see the media directory mounted at different paths.

#### Scenario: Scanned video matched regardless of mount path

- **WHEN** a downloaded video's YouTube ID appears as a `youtube://` GUID on
  a Plex item, and Plex's file path for the item differs from yarrtube's
  local path for it
- **THEN** the sync pass still associates that Plex item with the yarrtube
  video

### Requirement: Collection membership converges to yarrtube's state

Each sync pass SHALL be an idempotent upsert that converges every
collection's membership toward the set of the playlist's/channel's currently
downloaded videos: members missing from the collection are added when Plex
has scanned them, and collection members that no longer correspond to a
downloaded video of that playlist/channel are removed. Videos not yet
downloaded, or downloaded but not yet scanned by Plex, SHALL be picked up by
a later pass rather than treated as errors.

#### Scenario: Newly scanned video added on a later pass

- **WHEN** a video was downloaded after the previous sync pass and Plex has
  scanned it by the time the next pass runs
- **THEN** the next pass adds it to its playlist's/channel's collection

#### Scenario: Removed video leaves the collection

- **WHEN** a video is no longer part of a channel's tracked state (for
  example it slid out of the channel's video limit) and a sync pass runs
- **THEN** the pass removes the corresponding item from the channel's
  collection

#### Scenario: Pass is idempotent

- **WHEN** a sync pass runs twice in a row with no state changes in between
- **THEN** the second pass performs no collection mutations

### Requirement: Sync is decoupled from reconciliation

The Plex sync SHALL run as its own recurring global task covering all
playlists and channels in one pass. Reconcile passes MUST NOT perform Plex
requests or be delayed by the Plex integration in any way.

#### Scenario: Plex server unreachable

- **WHEN** the Plex server is down while reconcile and sync run
- **THEN** reconciliation, downloads and cleanup proceed unaffected, the
  sync pass fails with a logged error, and a next sync pass is scheduled

### Requirement: Per-collection failures do not abort the pass

Within a sync pass, a failure while syncing one playlist's/channel's
collection SHALL be logged and skipped, and the pass SHALL continue with the
remaining collections.

#### Scenario: One collection fails, others sync

- **WHEN** syncing one collection fails (for example a Plex error on its
  create call) during a pass over several playlists/channels
- **THEN** the remaining collections are still synced in the same pass

### Requirement: Deleting a playlist or channel deletes its collection

When a playlist or channel is deleted, the system SHALL delete its Plex
collection from every configured library section where one exists. A
missing collection MUST NOT be treated as an error.

#### Scenario: Playlist deleted

- **WHEN** a tracked playlist with an existing Plex collection is deleted
- **THEN** its Plex collection is deleted

#### Scenario: Deletion with integration disabled

- **WHEN** a playlist or channel is deleted while the Plex integration is
  disabled
- **THEN** deletion completes exactly as today, with no Plex requests
