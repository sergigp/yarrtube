# plex-category-labels Specification

## Purpose

Mirrors each stored video's categories into its Plex episode's labels and keeps one smart playlist per category and Plex library, so every category can be watched in Plex as a cross-show, self-updating feed.

## Requirements

### Requirement: Plex Labels Are Optional
The system SHALL label episodes and manage playlists in Plex only when `YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` and `YARRTUBE_PLEX_VIDEOS_PATH` are all set. Otherwise it MUST NOT contact any Plex server, and categories keep working in yarrtube unchanged.

#### Scenario: Integration not configured
- **WHEN** the daemon runs with any of the three variables unset and a category's members are set
- **THEN** no request is made to a Plex server

### Requirement: Episode Labels Mirror Video Categories
For every downloaded video that Plex has imported, the system SHALL make its Plex episode's labels exactly the names of the categories the video belongs to, removing any other label. A video in no category SHALL have no labels. Label names SHALL be compared ignoring case.

#### Scenario: Video in two categories
- **WHEN** a downloaded video of the channel "@GothamChess" belongs to "Chess" and "Funny", and a label sync runs
- **THEN** its Plex episode's labels are "Chess" and "Funny"

#### Scenario: Video in no category
- **WHEN** a downloaded video belongs to no category and its Plex episode has the label "Chess"
- **THEN** after a label sync its episode has no labels

#### Scenario: Copies of one YouTube video
- **WHEN** a YouTube video is tracked by a channel in "Chess" and by a playlist in no category
- **THEN** the channel's episode is labelled "Chess" and the playlist's episode has no labels

#### Scenario: Labels already matching
- **WHEN** a video belongs to "chess" and its episode is already labelled "Chess"
- **THEN** a label sync makes no change to that episode

#### Scenario: Label holding a comma
- **WHEN** a video leaves the category "Kids, Family"
- **THEN** after the next label sync its episode no longer has that label

### Requirement: Labels Follow Category Changes
When the categories of stored videos change, or a category they belong to is renamed or deleted, the system SHALL update their episodes' labels by a label sync that starts within a minute.

#### Scenario: Source added to a category
- **WHEN** the channel "@GothamChess" with 3 downloaded, imported videos is added to "Chess"
- **THEN** within a minute a label sync labels those 3 episodes "Chess"

#### Scenario: Category renamed
- **WHEN** the category "Funny" is renamed "Humor"
- **THEN** after the next label sync its videos' episodes are labelled "Humor" and no longer "Funny"

#### Scenario: Category deleted
- **WHEN** the category "Funny" is deleted
- **THEN** after the next label sync no episode is labelled "Funny"

#### Scenario: Change during a running sync
- **WHEN** a video's categories change while a label sync is labelling it
- **THEN** a later label sync labels its episode with the new categories

### Requirement: New Downloads Are Labelled
After a video downloads, the system SHALL label its episode once Plex has imported it. A label sync SHALL run about a minute after each download, and an episode not yet imported SHALL be labelled by a later sync without being reported as a failure.

#### Scenario: Episode imported after the scan
- **WHEN** a video of a channel in "Chess" downloads and Plex imports it before the label sync after the download
- **THEN** that sync labels its episode "Chess"

#### Scenario: Episode not imported yet
- **WHEN** a label sync runs before Plex has imported a downloaded video's episode
- **THEN** the video is labelled by a later sync once Plex has imported it, and no error is reported

#### Scenario: Video not downloaded yet
- **WHEN** a video of a categorized channel is still pending download
- **THEN** no label is written for it until it is downloaded and imported

### Requirement: Label Sync Never Blocks Downloads
Label syncs SHALL run in their own task lane, one at a time, so they never delay downloads, thumbnail fetches, subtitle fetches or playlist and channel syncs.

#### Scenario: Slow Plex during a download
- **WHEN** a label sync is waiting on a slow Plex server and a video download is due
- **THEN** the download starts without waiting for the label sync

### Requirement: Label Sync Survives Plex Failures
When Plex cannot be reached or rejects a request, the system SHALL keep the affected videos pending and apply their labels on a later sync. A failure for one episode SHALL NOT prevent the others in the same sync from being labelled.

#### Scenario: Plex down
- **WHEN** a label sync runs while the Plex server is unreachable
- **THEN** the sync ends without labelling anything, the videos stay pending, and a later sync labels them once Plex is back

#### Scenario: One episode rejected
- **WHEN** Plex rejects the label request for one of two pending episodes
- **THEN** the other episode is labelled and the rejected one is retried by a later sync

### Requirement: Periodic Label Sync Heals Drift
The system SHALL run a label sync at least every 15 minutes while the integration is configured. It SHALL label categorized, downloaded videos never labelled before, relabel episodes Plex re-imported under a new identity, and stop tracking deleted videos.

#### Scenario: Upgrade with existing categories
- **WHEN** the daemon starts after this change with Plex configured and 40 downloaded, imported videos in "Chess"
- **THEN** within 15 minutes all 40 episodes are labelled "Chess"

#### Scenario: Episode re-imported by Plex
- **WHEN** Plex re-imports a labelled video's file under a new identity, for example after its show folder moved
- **THEN** the next periodic label sync labels the new episode

#### Scenario: Large backlog
- **WHEN** more pending videos can be labelled than one sync handles
- **THEN** another sync runs right after it until they are labelled

#### Scenario: Backlog not labellable yet
- **WHEN** a sync takes as many pending videos as it handles and none of them can be labelled yet (not downloaded or not imported)
- **THEN** the next sync runs 15 minutes later, not right after

### Requirement: Smart Playlist Per Category And Library
For each category with at least one labelled episode in a Plex library, the system SHALL keep one smart playlist in that library, owned by the token's Plex user, listing that library's unwatched episodes labelled with the category, newest air date first.

#### Scenario: Category with episodes in one library
- **WHEN** episodes labelled "Chess" exist only in the library "Subscriptions"
- **THEN** one smart playlist titled "Chess" exists for "Subscriptions", listing its unwatched "Chess" episodes newest first

#### Scenario: Watched episode leaves the playlist
- **WHEN** an episode of the "Chess" playlist is watched in Plex
- **THEN** Plex no longer lists it in the playlist, without any action by yarrtube

#### Scenario: Category spanning libraries
- **WHEN** episodes labelled "Chess" exist in the libraries "Subscriptions" and "Sergi's Playlists"
- **THEN** one playlist exists for each, titled "Chess · Subscriptions" and "Chess · Sergi's Playlists"

### Requirement: Playlists Follow Their Categories
The system SHALL replace a category's playlists after it is renamed, delete a playlist when its category has no labelled episode left in that library, and recreate a playlist deleted in Plex, all during the label sync after the change.

#### Scenario: Category renamed
- **WHEN** the category "Funny" with a playlist is renamed "Humor"
- **THEN** after the next label sync the "Funny" playlist is gone and a "Humor" playlist lists the same episodes

#### Scenario: Category deleted
- **WHEN** the category "Funny" with a playlist is deleted
- **THEN** after the next label sync its playlist is gone

#### Scenario: Episodes gone from Plex
- **WHEN** every "Chess" episode of a library disappears from Plex while their videos stay in "Chess"
- **THEN** the next label sync deletes that library's "Chess" playlist

#### Scenario: Category renamed by case only
- **WHEN** the category "chess" with a playlist is renamed "Chess"
- **THEN** after the label syncs that relabel its episodes exactly one playlist, titled "Chess", exists for it in each library

#### Scenario: Playlist deleted in Plex
- **WHEN** a user deletes the "Chess" playlist in Plex while "Chess" still has labelled episodes
- **THEN** the next label sync creates it again

#### Scenario: Other playlists untouched
- **WHEN** the Plex user has playlists yarrtube did not create
- **THEN** no label sync changes or deletes them
