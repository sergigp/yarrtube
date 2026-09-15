## Purpose

Lets a caller create, delete, and list the YouTube channels the daemon tracks by handle, so a future sync capability has a persisted set of channels to poll for new uploads.

## ADDED Requirements

### Requirement: Create Channel
The system SHALL provide an HTTP endpoint that creates a channel given a `channel` value that is either a YouTube channel handle (e.g. `@somechannel`) or a YouTube channel URL carrying a handle (e.g. `https://www.youtube.com/@somechannel`), a quality tier (`high`, `mid`, or `low`), and a video limit that is a positive integer number of the channel's most recent videos to keep synced. The system SHALL extract the handle from the `channel` value when it is a URL, resolve that handle against the YouTube Data API to confirm it corresponds to an existing, accessible channel and to obtain that channel's immutable YouTube channel ID and display title, and store the channel with the handle as its ID, the resolved title as its name, the resolved immutable channel ID, the quality, the video limit, and a system-generated creation timestamp.

The stored ID is the handle, not the immutable channel ID, so the HTTP surface stays human-readable (e.g. `DELETE /channels/@somechannel`). The immutable ID is stored alongside it because a handle can be changed later by the channel's owner, while the immutable ID cannot.

#### Scenario: Successful creation from a bare handle
- **WHEN** a request supplies a `channel` value that is a bare handle that resolves to an existing YouTube channel, a valid quality, and a valid video limit
- **THEN** the system persists a channel record with that handle as its ID, the channel's resolved title as its name, the resolved immutable channel ID, the quality, the video limit, and a creation timestamp, and returns the created channel

#### Scenario: Successful creation from a channel URL
- **WHEN** a request supplies a `channel` value that is a full YouTube channel URL (e.g. `https://www.youtube.com/@somechannel`) whose handle resolves to an existing YouTube channel, a valid quality, and a valid video limit
- **THEN** the system extracts the handle from the URL, persists a channel record the same way as the bare-handle case, and returns the created channel

#### Scenario: Duplicate channel handle
- **WHEN** a request supplies a `channel` value (bare handle or URL) whose handle already exists in storage
- **THEN** the system accepts the request but makes no change because the endpoint is idempotent, and returns the existing channel record

#### Scenario: Duplicate channel handle with a different quality
- **WHEN** a request supplies a `channel` value whose handle already exists in storage, with a quality value different from the stored record's
- **THEN** the system makes no change, ignores the request's quality value, and returns the existing record with its original quality

#### Scenario: Duplicate channel handle with a different video limit
- **WHEN** a request supplies a `channel` value whose handle already exists in storage, with a video limit different from the stored record's
- **THEN** the system makes no change, ignores the request's video limit, and returns the existing record with its original video limit

#### Scenario: Missing or empty channel value
- **WHEN** a request omits the `channel` field, or supplies an empty or whitespace-only value
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Handle missing its leading @
- **WHEN** a request supplies a bare `channel` value that does not start with `@`
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Unrecognized URL
- **WHEN** a request supplies a `channel` value that is a URL but not a recognized YouTube URL
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: YouTube URL missing a handle
- **WHEN** a request supplies a `channel` value that is a recognized YouTube URL with no handle segment (e.g. a legacy `/channel/UC...` URL)
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Nonexistent YouTube channel
- **WHEN** a request supplies a `channel` value whose extracted handle does not correspond to an existing, accessible YouTube channel
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

#### Scenario: Missing or invalid quality
- **WHEN** a request omits quality, or supplies a value that is not `high`, `mid`, or `low`
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Missing or invalid video limit
- **WHEN** a request omits the video limit, or supplies a value that is not a positive integer
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

### Requirement: Delete Channel
The system SHALL provide an HTTP endpoint that deletes a previously created channel identified by its handle.

#### Scenario: Successful deletion
- **WHEN** a request identifies a channel handle that exists in storage
- **THEN** the system removes it and confirms the deletion

#### Scenario: Deleting a nonexistent channel
- **WHEN** a request identifies a channel handle that does not exist in storage
- **THEN** the system reports that nothing was found, makes no change to storage, and returns a bad request with a meaningful error description

### Requirement: List Channels
The system SHALL provide an HTTP endpoint that returns every currently stored channel.

#### Scenario: Channels exist
- **WHEN** one or more channels have been created
- **THEN** the system returns all of them, each with its handle, name, immutable YouTube channel ID, quality, video limit, and creation timestamp

#### Scenario: No channels exist
- **WHEN** no channels have been created
- **THEN** the system returns an empty list rather than an error

### Requirement: Channel Creation Publishes a Domain Event
The system SHALL publish a ChannelCreated domain event, containing the channel's handle, whenever a channel is newly created — and SHALL NOT publish it when creation is a no-op because the channel already existed.

#### Scenario: New channel created
- **WHEN** a create-channel request results in a new channel being persisted
- **THEN** the system publishes a ChannelCreated event containing that channel's handle

#### Scenario: Channel already existed
- **WHEN** a create-channel request identifies a handle that already exists in storage
- **THEN** the system does not publish a ChannelCreated event

### Requirement: Channel Deletion Publishes a Domain Event
The system SHALL publish a ChannelDeleted domain event, containing the channel's handle, whenever a channel is successfully deleted.

#### Scenario: Existing channel deleted
- **WHEN** a delete-channel request successfully removes a channel from storage
- **THEN** the system publishes a ChannelDeleted event containing that channel's handle

#### Scenario: Deleting a nonexistent channel
- **WHEN** a delete-channel request identifies a handle that does not exist in storage
- **THEN** the system does not publish a ChannelDeleted event
