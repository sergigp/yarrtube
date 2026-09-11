## Purpose

Keeps every tracked playlist's stored videos and downloaded files converged
onto their declared desired state — YouTube's membership for a YouTube-linked
playlist, and the database's own record of what should be on disk for every
playlist — through a recurring, self-healing reconcile pass rather than a
one-shot sync.

## ADDED Requirements

### Requirement: Reconcile On Playlist Creation
The system SHALL run one reconcile pass for a playlist as soon as practical
after it is created, regardless of the playlist's kind.

#### Scenario: Newly created YouTube-linked playlist gets reconciled
- **WHEN** a YouTube-linked playlist is successfully created
- **THEN** the system fetches its current members from the YouTube Data API and persists them

#### Scenario: Newly created custom playlist gets reconciled
- **WHEN** a custom playlist is successfully created
- **THEN** the system runs a reconcile pass that makes no YouTube API request and completes as a no-op, since the playlist has no videos yet

### Requirement: Recurring Reconciliation
The system SHALL schedule the next reconcile of a playlist after every
reconcile attempt that runs to completion, at a configurable interval,
regardless of the playlist's kind, whether any videos were added or removed,
or whether any file was healed.

#### Scenario: Reconcile finds no changes
- **WHEN** a playlist reconcile completes and finds no membership or filesystem changes
- **THEN** the system still schedules the next reconcile of that playlist after the configured interval

#### Scenario: Reconcile finds changes
- **WHEN** a playlist reconcile completes and finds membership and/or filesystem changes
- **THEN** the system schedules the next reconcile of that playlist after the configured interval, in addition to persisting the changes

### Requirement: Configurable Reconcile Interval
The system SHALL read the recurring reconcile interval from a configuration
value, defaulting to 3600 seconds when it is not set.

#### Scenario: Interval configured
- **WHEN** a reconcile interval configuration value is present
- **THEN** the system uses it as the delay before the next reconcile

#### Scenario: Interval not configured
- **WHEN** no reconcile interval configuration value is present
- **THEN** the system defaults to a 3600 second delay before the next reconcile

### Requirement: YouTube Membership Diff Applies Only To YouTube-Linked Playlists
The system SHALL fetch and diff a playlist's membership against YouTube only
when the playlist's kind is YouTube-linked. It SHALL NOT make any YouTube API
request for a custom playlist's reconcile pass.

#### Scenario: YouTube-linked playlist reconciled
- **WHEN** a reconcile pass runs for a YouTube-linked playlist
- **THEN** the system fetches its current members from the YouTube Data API and diffs them against stored videos

#### Scenario: Custom playlist reconciled
- **WHEN** a reconcile pass runs for a custom playlist
- **THEN** the system makes no YouTube API request and does not alter the playlist's video membership

### Requirement: New Video Persistence (YouTube-Linked Playlists)
The system SHALL store each video found in a YouTube-linked playlist that is
not already stored for that playlist, with a status of PENDING, and SHALL
publish a `VideoAdded` event for it.

#### Scenario: Playlist contains a video not yet stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a video in the playlist that has no existing stored record for that playlist
- **THEN** the system stores it with status PENDING and publishes a `VideoAdded` event

#### Scenario: Playlist contains a video already stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a video that already has a stored record for that playlist
- **THEN** the system leaves that record's status unchanged and does not publish another `VideoAdded` event

### Requirement: Removed Video Cleanup (YouTube-Linked Playlists)
The system SHALL delete a stored video's record when a reconcile pass for a
YouTube-linked playlist finds that it is no longer present in the source
YouTube playlist, SHALL log each such deletion, and SHALL publish a
`VideoDeleted` event carrying that video's title, its recorded filename (if
any), and whether it had been downloaded.

#### Scenario: Previously stored video no longer in the YouTube playlist
- **WHEN** a reconcile pass finds that a video previously stored for a YouTube-linked playlist is no longer a member of that playlist on YouTube
- **THEN** the system deletes its stored record and logs the deletion

#### Scenario: Deletion publishes a VideoDeleted event
- **WHEN** a reconcile pass deletes a stored video's record because it is no longer in the source YouTube playlist
- **THEN** the system publishes a `VideoDeleted` event containing the playlist ID, the video ID, the video's title, its recorded filename (if any), and whether the video had been downloaded

### Requirement: Reconcile Skipped For a Deleted Playlist
The system SHALL NOT contact YouTube, persist any videos, sweep the
filesystem, or schedule a further reconcile when a reconcile pass runs for a
playlist that no longer exists.

#### Scenario: Scheduled reconcile runs after its playlist was deleted
- **WHEN** a reconcile pass is attempted for a playlist ID that no longer exists in storage
- **THEN** the system makes no YouTube request, persists nothing, performs no filesystem sweep, and does not schedule another reconcile for that playlist ID

### Requirement: Filesystem Reconciliation Against Recorded Downloads
The system SHALL, for every playlist regardless of kind, compare the files
present in that playlist's output directory against the recorded filename of
each of its Downloaded videos, and heal any divergence it finds.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its playlist's output directory
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: A file on disk is not accounted for by any Downloaded video
- **WHEN** a reconcile pass finds a file in the playlist's output directory that is not the recorded filename of any currently Downloaded video, and is not a downloader-managed in-progress temporary file
- **THEN** the system deletes that file

#### Scenario: A downloader-managed in-progress temporary file is present
- **WHEN** a reconcile pass finds a file in the playlist's output directory that the downloader itself uses to track an in-progress download
- **THEN** the system does not treat that file as orphaned and does not delete it

#### Scenario: Downloaded video's file is present
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its playlist's output directory
- **THEN** the system takes no action for that video
