## Purpose

Lets a caller create, delete, and list the playlists the daemon tracks, so a future scheduler has a persisted set of playlists to query and download from.

## Requirements

### Requirement: Create Playlist
The system SHALL provide an HTTP endpoint that creates a playlist given a `playlist` value that is either a YouTube playlist ID or a YouTube playlist URL, a storage path, a quality tier (`high`, `mid`, or `low`), and an optional `exclude_from_home` boolean that defaults to `false` when omitted. The request SHALL NOT carry a name. The system SHALL extract the playlist ID from the `playlist` value when it is a URL, and SHALL look the resulting YouTube playlist ID up on YouTube before persisting, confirming that it corresponds to an existing, accessible YouTube playlist. It SHALL store the playlist with kind `youtube_linked`, that ID, the playlist's YouTube title as its name, the path, the quality, the `exclude_from_home` setting, and a system-generated creation timestamp. When YouTube reports an empty or blank title, the system SHALL use the playlist ID as the name.

The storage path identifies where the playlist's videos are saved, relative to the configured videos root directory, and may contain multiple `/`-separated segments to express nested subdirectories (e.g. `music/chill`).

The name is a display name only and never names a directory. It SHALL be accepted whatever characters it contains, including `/`, `:` and `?`, as long as it is not blank.

Every playlist created through this endpoint has kind `youtube_linked`.

#### Scenario: Successful creation
- **WHEN** a request supplies a `playlist` value that is a bare YouTube playlist ID that exists on YouTube with the title "Lofi beats", a valid path, and a valid quality
- **THEN** the system persists a playlist record with that ID, the name "Lofi beats", that path and quality, kind `youtube_linked`, and a creation timestamp, and returns the created playlist including its name and kind

#### Scenario: Successful creation from a YouTube URL
- **WHEN** a request supplies a `playlist` value that is a full YouTube playlist URL (e.g. `https://www.youtube.com/playlist?list=PLabc123`) whose extracted playlist ID exists on YouTube, a valid path, and a valid quality
- **THEN** the system extracts the playlist ID from the URL, persists a playlist record with that ID, its YouTube title as the name, that path and quality, kind `youtube_linked`, and a creation timestamp, and returns the created playlist including its kind

#### Scenario: Created playlist shown on home by default
- **WHEN** a valid creation request omits `exclude_from_home`
- **THEN** the system persists the playlist with `exclude_from_home` set to `false`, and returns it with `exclude_from_home` `false`

#### Scenario: Creating a playlist excluded from home
- **WHEN** a valid creation request supplies `exclude_from_home` `true`
- **THEN** the system persists the playlist with `exclude_from_home` set to `true`, and returns it with `exclude_from_home` `true`

#### Scenario: Invalid exclude from home value
- **WHEN** a request supplies an `exclude_from_home` value that is not a boolean
- **THEN** the system rejects the request with a client error, without persisting anything and without checking YouTube

#### Scenario: YouTube title with filesystem-unsafe characters
- **WHEN** a request identifies a YouTube playlist whose title is "AC/DC: greatest hits?"
- **THEN** the system persists the playlist with the name "AC/DC: greatest hits?", and lists it under that name afterwards

#### Scenario: YouTube playlist with a blank title
- **WHEN** a request identifies a YouTube playlist whose title YouTube reports as empty
- **THEN** the system persists the playlist with its playlist ID as the name

#### Scenario: Invalid name
- **WHEN** a request supplies a `name` field, whether empty, containing filesystem-unsafe characters, or valid
- **THEN** the system neither rejects the request because of it nor uses it, and names the playlist after its YouTube title

#### Scenario: Duplicate playlist ID
- **WHEN** a request supplies a `playlist` value (bare ID or URL) whose playlist ID already exists in storage
- **THEN** the system accepts the request but makes no change bc the endpoint is idempotent, and returns the existing playlist record

#### Scenario: Duplicate playlist ID with a different quality
- **WHEN** a request supplies a `playlist` value whose playlist ID already exists in storage, with a quality value different from the stored record's
- **THEN** the system makes no change, ignores the request's quality value, and returns the existing record with its original quality

#### Scenario: Duplicate playlist ID with a different path
- **WHEN** a request supplies a `playlist` value whose playlist ID already exists in storage, with a path value different from the stored record's
- **THEN** the system makes no change, ignores the request's path value, and returns the existing record with its original path

#### Scenario: Duplicate playlist ID with a different exclude from home value
- **WHEN** a request supplies a `playlist` value whose playlist ID already exists in storage, with an `exclude_from_home` value different from the stored record's
- **THEN** the system makes no change, ignores the request's `exclude_from_home` value, and returns the existing record with its original setting

#### Scenario: Missing or invalid path
- **WHEN** a request omits path, supplies an empty path, or supplies a path that is absolute, contains a `..` segment, or contains an empty segment (e.g. leading/trailing/doubled `/`)
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Missing or invalid quality
- **WHEN** a request omits quality, or supplies a value that is not `high`, `mid`, or `low`
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Missing or empty playlist value
- **WHEN** a request omits the `playlist` field, or supplies an empty or whitespace-only value
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Unrecognized URL
- **WHEN** a request supplies a `playlist` value that is a URL but not a recognized YouTube URL
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: YouTube URL missing the playlist parameter
- **WHEN** a request supplies a `playlist` value that is a recognized YouTube URL (e.g. a video watch URL) that has no `list` query parameter
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Nonexistent YouTube playlist
- **WHEN** a request supplies a `playlist` value whose extracted playlist ID does not correspond to an existing, accessible YouTube playlist
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

### Requirement: Preview a YouTube Playlist
The system SHALL provide an HTTP endpoint that, given a `playlist` value that is either a YouTube playlist ID or a YouTube playlist URL, looks the playlist up on YouTube and returns its playlist ID, its YouTube title, and the number of videos YouTube reports for it, without persisting anything, publishing any event, or scheduling any task. It applies the same ID extraction and validation as Create Playlist, and the same fallback to the playlist ID when YouTube reports a blank title. The count is YouTube's own item count and may include videos that will not be downloaded, such as private ones.

The preview is independent of what is already tracked: previewing a playlist that is already stored SHALL return its YouTube data like any other.

#### Scenario: Previewing a playlist by ID
- **WHEN** a request supplies a bare playlist ID of a YouTube playlist titled "Lofi beats" with 42 videos
- **THEN** the system returns that playlist ID, the title "Lofi beats" and the count 42, and storage is unchanged

#### Scenario: Previewing a playlist by URL
- **WHEN** a request supplies a YouTube playlist URL (e.g. `https://www.youtube.com/watch?v=vid1&list=PLabc123`)
- **THEN** the system returns the extracted playlist ID `PLabc123` together with its YouTube title and video count

#### Scenario: Previewing a playlist with a blank title
- **WHEN** a request identifies a YouTube playlist whose title YouTube reports as empty
- **THEN** the system returns the playlist ID as the title

#### Scenario: Previewing an invalid value
- **WHEN** a request supplies an empty value, a URL that is not a recognized YouTube URL, or a YouTube URL without a `list` parameter
- **THEN** the system returns a bad request with a meaningful error description without contacting YouTube

#### Scenario: Previewing a nonexistent playlist
- **WHEN** a request supplies a playlist ID that does not correspond to an existing, accessible YouTube playlist
- **THEN** the system returns a not found response with a meaningful error description

#### Scenario: YouTube unavailable
- **WHEN** the YouTube lookup itself fails
- **THEN** the system returns a bad gateway response with a meaningful error description

### Requirement: Unique Playlist Paths
The system SHALL reject a request to create a playlist whose storage path is already used by a different existing playlist.

#### Scenario: Path already used by another playlist
- **WHEN** a create request supplies a path that is already the stored path of a different existing playlist
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

#### Scenario: Re-submitting the same playlist's own path
- **WHEN** a create request identifies a playlist ID that already exists and supplies that same playlist's own stored path
- **THEN** the system treats this as the existing idempotent duplicate-ID case and does not reject it under this rule

### Requirement: Delete a Playlist
The system SHALL provide an HTTP endpoint that deletes a previously created playlist identified by its ID, and SHALL delete every video record stored for that playlist as part of the same operation, so that no video record can ever be observed referencing a playlist that no longer exists. Deleting a playlist SHALL also remove its pending recurring reconcile, so no reconcile of the playlist stays queued after it is gone.

#### Scenario: Successful deletion
- **WHEN** a request identifies a playlist ID that exists in storage
- **THEN** the system removes it, removes every video record stored for it, and confirms the deletion

#### Scenario: Deleting a playlist with videos
- **WHEN** a request identifies a playlist ID that has one or more video records stored for it, regardless of their download status
- **THEN** by the time the system confirms the deletion, none of those video records remain in storage

#### Scenario: Deleting a playlist with a pending reconcile
- **WHEN** a request deletes a playlist whose next recurring reconcile is pending
- **THEN** by the time the system confirms the deletion, no reconcile of that playlist is pending, and pending tasks of other playlists and channels are unchanged

#### Scenario: Playlist added again after deletion
- **WHEN** a playlist is deleted and then created again with the same ID
- **THEN** the playlist has a single recurring reconcile chain, as if it had never been tracked before

#### Scenario: Deleting a nonexistent playlist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system reports that nothing was found and makes no change to storage, including no change to any video records, and returns a bad request with a meaningful error description

### Requirement: List All Playlists
The system SHALL provide an HTTP endpoint that returns every currently stored playlist, sorted alphabetically by name, ignoring case. Each returned playlist SHALL include its count of unwatched videos, counting only videos that have finished downloading, and whether that count is hidden.

#### Scenario: Playlists exist
- **WHEN** one or more playlists have been created
- **THEN** the system returns all of them, each with its ID, name, path, quality, kind, whether it is excluded from home, reconcile interval in minutes, creation timestamp, unwatched video count, and whether its unwatched count is hidden

#### Scenario: No playlists exist
- **WHEN** no playlists have been created
- **THEN** the system returns an empty list rather than an error

#### Scenario: Playlists are sorted by name regardless of creation order
- **WHEN** playlists named "watch later", "Courses" and "Ambient" were created in that order
- **THEN** the system returns them in the order "Ambient", "Courses", "watch later"

#### Scenario: Unwatched count only includes downloaded, unwatched videos
- **WHEN** a playlist has downloaded videos that are unwatched, downloaded videos that are watched, and videos that have not finished downloading
- **THEN** its unwatched video count equals the number of downloaded, unwatched videos only

#### Scenario: Hidden unwatched count is still returned
- **WHEN** a playlist with 2 downloaded, unwatched videos has its unwatched count hidden
- **THEN** the system returns it with unwatched video count 2 and `hide_unwatched_count` `true`

### Requirement: Playlist Creation Publishes a Domain Event
The system SHALL publish a PlaylistCreated domain event, containing the playlist's ID, whenever a playlist is newly created — and SHALL NOT publish it when creation is a no-op because the playlist already existed.

#### Scenario: New playlist created
- **WHEN** a create-playlist request results in a new playlist being persisted
- **THEN** the system publishes a PlaylistCreated event containing that playlist's ID

#### Scenario: Playlist already existed
- **WHEN** a create-playlist request identifies a playlist ID that already exists in storage
- **THEN** the system does not publish a PlaylistCreated event

### Requirement: Playlist Deletion Publishes an Event
The system SHALL publish a PlaylistDeleted domain event, containing the playlist's ID and its storage path, whenever a playlist is successfully deleted. The path travels with the event because the playlist record itself no longer exists by the time anything reacts to it.

#### Scenario: Existing playlist deleted
- **WHEN** a delete-playlist request successfully removes a playlist from storage
- **THEN** the system publishes a PlaylistDeleted event containing that playlist's ID and its storage path

#### Scenario: Deleting a nonexistent playlist
- **WHEN** a delete-playlist request identifies a playlist ID that does not exist in storage
- **THEN** the system does not publish a PlaylistDeleted event

### Requirement: Playlist Reconcile Interval
Every stored playlist SHALL have a reconcile interval: a whole number of minutes from 1 to 1440 (24 hours), setting how long after one recurring reconcile of the playlist finishes the next one runs. Creating a playlist SHALL accept an optional `reconcile_interval_minutes`; when it is omitted the playlist is stored with 180 minutes (3 hours). Playlists stored before reconcile intervals existed SHALL have 180 minutes. The created playlist's response SHALL include its `reconcile_interval_minutes`.

#### Scenario: Creating a playlist without an interval
- **WHEN** a valid creation request omits `reconcile_interval_minutes`
- **THEN** the system persists the playlist with a reconcile interval of 180 minutes, and returns it with `reconcile_interval_minutes` 180

#### Scenario: Creating a playlist with an interval
- **WHEN** a valid creation request supplies `reconcile_interval_minutes` 720
- **THEN** the system persists the playlist with a reconcile interval of 720 minutes, and returns it with `reconcile_interval_minutes` 720

#### Scenario: Invalid interval on creation
- **WHEN** a creation request supplies a `reconcile_interval_minutes` that is not a whole number from 1 to 1440
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request whose error description states the bound it breaks (at least 1 minute, at most 1440 minutes) and the value received

#### Scenario: Duplicate playlist ID with a different interval
- **WHEN** a request supplies a `playlist` value whose playlist ID already exists in storage, with a `reconcile_interval_minutes` different from the stored record's
- **THEN** the system makes no change, ignores the request's interval, and returns the existing record with its original interval

#### Scenario: Playlists existing before the upgrade
- **WHEN** the system starts on a database whose playlists were stored before reconcile intervals existed
- **THEN** every such playlist has a reconcile interval of 180 minutes

### Requirement: Update Playlist Settings
The system SHALL provide an HTTP endpoint that changes whether a previously created playlist, identified by its ID, is excluded from home, whether its unwatched count is hidden, and/or its reconcile interval, given an optional `exclude_from_home` boolean, an optional `hide_unwatched_count` boolean and an optional `reconcile_interval_minutes`, at least one of which SHALL be supplied. An absent field keeps its stored value. It SHALL change only those settings, leaving the playlist's other fields and its video records as they were, and SHALL return the updated playlist. Setting the values the playlist already has SHALL succeed without changing anything. A new reconcile interval SHALL apply from the next scheduled reconcile on: when the playlist's pending recurring reconcile is due later than the update time plus the new interval, the system SHALL move it to the update time plus the new interval; otherwise the pending reconcile keeps its time. The update publishes no domain event.

#### Scenario: Excluding a playlist from home
- **WHEN** a request sets `exclude_from_home` to `true` on a playlist that is shown on home
- **THEN** the system stores the playlist as excluded from home, returns it with `exclude_from_home` `true`, and leaves its name, path, quality, kind, reconcile interval, creation timestamp and video records unchanged

#### Scenario: Including a playlist in home again
- **WHEN** a request sets `exclude_from_home` to `false` on a playlist that is excluded from home
- **THEN** the system stores the playlist as shown on home, and returns it with `exclude_from_home` `false`

#### Scenario: Hiding a playlist's unwatched count
- **WHEN** a request sets `hide_unwatched_count` to `true` on a playlist whose unwatched count is shown
- **THEN** the system stores the playlist with its unwatched count hidden, returns it with `hide_unwatched_count` `true`, and leaves its other fields and video records unchanged

#### Scenario: Showing a playlist's unwatched count again
- **WHEN** a request sets `hide_unwatched_count` to `false` on a playlist whose unwatched count is hidden
- **THEN** the system stores the playlist with its unwatched count shown, and returns it with `hide_unwatched_count` `false`

#### Scenario: Changing the reconcile interval
- **WHEN** a request supplies `reconcile_interval_minutes` 60 for a playlist whose interval is 180
- **THEN** the system stores the playlist with a reconcile interval of 60 minutes, returns it, and leaves its other fields and video records unchanged

#### Scenario: Shortening the interval brings the pending reconcile forward
- **WHEN** a playlist's pending recurring reconcile is due in 20 hours and a request changes its interval to 60 minutes
- **THEN** the pending reconcile becomes due 60 minutes after the update

#### Scenario: Lengthening the interval keeps the pending reconcile
- **WHEN** a playlist's pending recurring reconcile is due in 30 minutes and a request changes its interval to 1440 minutes
- **THEN** the pending reconcile stays due in 30 minutes, and the reconcile after it is scheduled 1440 minutes after that one finishes

#### Scenario: Changing the interval while a reconcile is running
- **WHEN** a request changes a playlist's interval while a reconcile of that playlist is running and none is pending
- **THEN** the running reconcile, when it finishes, schedules the next one after the new interval

#### Scenario: Setting the values the playlist already has
- **WHEN** a request sets `exclude_from_home`, `hide_unwatched_count` and `reconcile_interval_minutes` to the values the playlist already has
- **THEN** the system succeeds and returns the playlist unchanged

#### Scenario: Unknown playlist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system makes no change and responds with not found and a meaningful error description

#### Scenario: Nothing to update
- **WHEN** a request supplies none of `exclude_from_home`, `hide_unwatched_count` and `reconcile_interval_minutes`
- **THEN** the system makes no change and returns a bad request with a meaningful error description

#### Scenario: Invalid exclude from home value
- **WHEN** a request supplies an `exclude_from_home` value that is not a boolean
- **THEN** the system makes no change and rejects the request with a client error

#### Scenario: Invalid hide unwatched count value
- **WHEN** a request supplies a `hide_unwatched_count` value that is not a boolean
- **THEN** the system makes no change and rejects the request with a client error

#### Scenario: Invalid interval
- **WHEN** a request supplies a `reconcile_interval_minutes` that is not a whole number from 1 to 1440
- **THEN** the system makes no change and returns a bad request with the same error description as on creation

### Requirement: Playlist Unwatched Count Visibility
The system SHALL record, for each playlist, whether its unwatched count is hidden. A newly created playlist SHALL have it shown. The setting SHALL NOT change how the playlist's own unwatched count is computed or whether it is returned; clients use it to decide whether to display that count. A playlist whose count is hidden SHALL add nothing to category unwatched counts.

#### Scenario: Created playlist shows its unwatched count
- **WHEN** a playlist is created
- **THEN** it is stored and returned with `hide_unwatched_count` `false`

#### Scenario: Playlists existing before the upgrade
- **WHEN** the system starts on a database whose playlists were stored before this setting existed
- **THEN** every such playlist has `hide_unwatched_count` `false`
