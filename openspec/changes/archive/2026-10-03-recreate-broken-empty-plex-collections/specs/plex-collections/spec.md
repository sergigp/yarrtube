## MODIFIED Requirements

### Requirement: Collection membership converges to yarrtube's state

Each sync pass SHALL be an idempotent upsert that converges every
collection's membership toward the set of the playlist's/channel's currently
downloaded videos: members missing from the collection are added when Plex
has scanned them, and collection members that no longer correspond to a
downloaded video of that playlist/channel are removed. Videos not yet
downloaded, or downloaded but not yet scanned by Plex, SHALL be picked up by
a later pass rather than treated as errors.

When an existing collection has no members and the playlist/channel has
scanned videos to put in it, the pass SHALL delete that collection and create
it again with those videos (with alphabetical sorting configured) instead of
adding to it, so a collection Plex no longer accepts additions to recovers on
its own.

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

#### Scenario: Empty collection is recreated with its videos

- **WHEN** a sync pass runs and a playlist's/channel's collection exists in
  the section with no members, and the playlist/channel has downloaded videos
  Plex has scanned there
- **THEN** the pass deletes that collection and creates a collection with the
  same name in that section containing those videos, with alphabetical
  sorting configured

#### Scenario: Empty collection left alone when there is nothing to add

- **WHEN** a sync pass runs and a playlist's/channel's collection exists with
  no members, and none of its downloaded videos are scanned in that section
- **THEN** the pass performs no mutation on that collection
