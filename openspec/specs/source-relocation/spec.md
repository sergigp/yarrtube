# source-relocation Specification

## Purpose

Lets a user move a tracked playlist's or channel's show directory to a new storage path under the videos root, keeping every video, episode number and watch state, without re-downloading anything.

## Requirements

### Requirement: Move a Playlist
The system SHALL provide an HTTP endpoint that moves a playlist, identified by its ID, to a new storage `path`. On success the playlist's show directory and everything in it SHALL be at the new path, the playlist SHALL be stored with the new path, and the endpoint SHALL return the playlist as stored. Video records, episode numbers, watch state, settings and the pending recurring reconcile SHALL be unchanged.

#### Scenario: Successful playlist move
- **WHEN** a settled playlist stored at `music/chill` with downloaded videos is moved to `music/lofi`
- **THEN** its `tvshow.nfo`, season folders and every episode file are under `music/lofi`, nothing remains at `music/chill`, the playlist is returned and listed with path `music/lofi`, and its videos still play and keep their watch state

#### Scenario: Moving into a parent that does not exist yet
- **WHEN** a settled playlist is moved to `archive/2026/chill` and `archive/2026` does not exist
- **THEN** the missing parent directories are created and the move succeeds

#### Scenario: Pending reconcile kept
- **WHEN** a settled playlist whose next recurring reconcile is pending is moved
- **THEN** that reconcile stays pending at its original time, and when it runs it reconciles the files at the new path without re-downloading anything

#### Scenario: Moving a nonexistent playlist
- **WHEN** a move request identifies a playlist ID that does not exist
- **THEN** the system returns not found and changes nothing

### Requirement: Move a Channel
The system SHALL provide an HTTP endpoint that moves a channel, identified by its handle, to a new storage `path`, with the same effects, preconditions and errors as moving a playlist. The channel's stored avatar SHALL be unaffected, and its `poster.jpg` SHALL move with the show directory.

#### Scenario: Successful channel move
- **WHEN** a settled channel stored at `channels/veritasium` is moved to `science/veritasium`
- **THEN** its `tvshow.nfo`, `poster.jpg`, season folders and every episode file are under `science/veritasium`, and the channel is returned and listed with that path

#### Scenario: Moving a nonexistent channel
- **WHEN** a move request identifies a channel handle that does not exist
- **THEN** the system returns not found and changes nothing

### Requirement: Move Requires A Settled Source
The system SHALL accept a move only when every video of the source is Downloaded, Errored or Excluded, and no download, thumbnail fetch, subtitle fetch or video file deletion task for any of its videos is pending or running, and no downloaded video of it is still awaiting its follow-up handling (the scheduling of its subtitle fetch). The source's pending recurring reconcile SHALL NOT block a move. Otherwise the system SHALL reject the move with a conflict, a message saying the source still has work in progress, and change nothing.

#### Scenario: Video still pending download
- **WHEN** a move is requested for a source with a video that is Pending, downloading, or errored with a retry still due
- **THEN** the system rejects it with a conflict and changes nothing

#### Scenario: Thumbnail or subtitle fetch queued
- **WHEN** every video of a source is Downloaded but a thumbnail or subtitle fetch for one of them is pending
- **THEN** the system rejects the move with a conflict and changes nothing

#### Scenario: Download just finished
- **WHEN** every video of a source is Downloaded and no task is queued, but a just-finished download's follow-up (scheduling its subtitle fetch) has not been handled yet
- **THEN** the system rejects the move with a conflict and changes nothing

#### Scenario: Errored and excluded videos do not block
- **WHEN** every video of a source is Downloaded except one permanently Errored and one Excluded video, and no per-video task is pending or running
- **THEN** the move succeeds

#### Scenario: Source with no videos
- **WHEN** a move is requested for a source that has no videos yet
- **THEN** the move succeeds

### Requirement: Listings Report Work In Progress
The playlist and channel listings SHALL report, for every source, whether it has work in progress, by exactly the rule a move is accepted under (see "Move Requires A Settled Source"), so a client can tell beforehand whether a move would be refused for it.

#### Scenario: Video still pending
- **WHEN** a source with a video that is not Downloaded, Errored or Excluded is listed
- **THEN** it is listed with work in progress

#### Scenario: Per-video task queued
- **WHEN** a source whose videos are all Downloaded, with a thumbnail or subtitle fetch queued in its show folder, is listed
- **THEN** it is listed with work in progress

#### Scenario: Download just finished
- **WHEN** a source whose videos are all Downloaded, with no task queued but a just-finished download's follow-up not yet handled, is listed
- **THEN** it is listed with work in progress

#### Scenario: Settled source
- **WHEN** a source whose videos are all Downloaded, Errored or Excluded, with no per-video task queued or running in its show folder, is listed
- **THEN** it is listed without work in progress

### Requirement: Move Destination Validation
The system SHALL reject a move whose path is invalid by the same rules as on creation, equals the current path, is the stored path of another playlist or channel, is inside or contains the stored path of any playlist or channel (itself included), is equal to, inside or contains the path of a source whose deletion is still pending, or already exists on disk. An invalid path SHALL be a bad request; any other rejection SHALL be a conflict. A rejected move changes nothing.

#### Scenario: Invalid path
- **WHEN** a move request supplies an empty or absolute path, or one containing a `.`, `..` or empty segment
- **THEN** the system returns a bad request and changes nothing

#### Scenario: Path used by another source
- **WHEN** a move request supplies a path that is the stored path of another playlist or channel
- **THEN** the system returns a conflict and changes nothing

#### Scenario: Path nested in the current path
- **WHEN** a source at `music` is moved to `music/chill`, or a source at `music/chill` is moved to `music`
- **THEN** the system returns a conflict and changes nothing

#### Scenario: Path nested in another source's path
- **WHEN** a source is moved to `music/chill/extra` while another playlist is stored at `music/chill`
- **THEN** the system returns a conflict and changes nothing

#### Scenario: Path inside a folder being deleted
- **WHEN** a playlist stored at `archive` was just deleted and its folder is not removed yet, and another source is moved to `archive/b`
- **THEN** the system returns a conflict and changes nothing

#### Scenario: Destination exists on disk
- **WHEN** a move request supplies a path where a file or directory already exists
- **THEN** the system returns a conflict and changes nothing, leaving the existing entry untouched

#### Scenario: Same path
- **WHEN** a move request supplies the source's current path
- **THEN** the system returns a conflict and changes nothing

### Requirement: Move Is A Same-Filesystem Rename
The system SHALL move a show directory by renaming it in place, never by copying. When the filesystem cannot rename it to the destination (e.g. the destination is on another mount) the system SHALL reject the move with nothing changed. If recording the new path fails after the rename, the system SHALL rename the directory back and report the error.

#### Scenario: Destination on another filesystem
- **WHEN** the destination lies on a different filesystem from the source's show directory
- **THEN** the system rejects the move with an error, the files stay at the old path and the source keeps its old path

#### Scenario: Recording the new path fails
- **WHEN** the rename succeeds but storing the new path fails
- **THEN** the directory is renamed back to the old path, the source keeps its old path, and the system returns an error

#### Scenario: Show directory missing on disk
- **WHEN** a settled source with no videos is moved and its show directory does not exist on disk
- **THEN** the system records the new path without renaming anything

### Requirement: Moves And Reconciles Never Overlap
The system SHALL never run a move and a reconcile of the same source at the same time. A reconcile of a source, recurring or on demand, that starts while that source is being moved SHALL wait for the move to finish and then reconcile the new path. A move requested while a reconcile of that source runs SHALL be rejected with a conflict, changing nothing.

#### Scenario: Move during a running reconcile
- **WHEN** a move is requested for a source whose reconcile is running
- **THEN** the system rejects the move with a conflict saying the source is being synced, and changes nothing

#### Scenario: Reconcile requested during a move
- **WHEN** an on-demand or recurring reconcile of a source starts while that source is being moved
- **THEN** the reconcile runs only after the move finishes, against the new path, and re-downloads nothing

#### Scenario: Other sources unaffected
- **WHEN** a source is being moved
- **THEN** reconciles of other sources run as usual

### Requirement: Plex Scan After A Move
When the Plex integration is enabled, the system SHALL, after a successful move, ask Plex to scan the source's new show folder and its old one, following the same section discovery, asynchrony, retry and logging rules as the scan after a download. A failed scan request SHALL NOT affect the move's outcome.

#### Scenario: Plex enabled
- **WHEN** a source is moved from `music/chill` to `music/lofi` with the Plex integration enabled
- **THEN** Plex is asked to scan both folders' Plex-side paths in every section containing them

#### Scenario: Plex disabled
- **WHEN** a source is moved with the Plex integration disabled
- **THEN** no request is made to a Plex server
