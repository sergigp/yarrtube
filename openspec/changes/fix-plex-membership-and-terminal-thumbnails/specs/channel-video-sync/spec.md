## MODIFIED Requirements

### Requirement: Missing Thumbnail Recovery (Channels)
The system SHALL, for every channel, schedule a thumbnail fetch task during
a reconcile pass for each video that has no recorded thumbnail filename,
regardless of its download status except as listed below, without affecting that video's download
status or triggering a redownload. The pass SHALL NOT fetch the thumbnail
itself, and SHALL NOT schedule one for:
- a video that already has a thumbnail fetch pending or running,
- a video the same pass resets for redownload,
- a video whose download is In Progress,
- a video whose download status is Excluded (permanently unavailable, so a fetch can never succeed) or Errored (recovery later resets it for redownload, and that download writes its own thumbnail),
- a video the same pass newly added: its thumbnail fetch is scheduled in reaction to its `VideoAddedToChannel` event (see `video-thumbnails`), so recovery only covers videos already stored before the pass.

#### Scenario: Video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel with no recorded thumbnail filename and no thumbnail fetch pending or running for it
- **THEN** the system schedules a thumbnail fetch task for that video

#### Scenario: Video already has a recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel that already has a recorded thumbnail filename
- **THEN** the system does not schedule a thumbnail fetch for it

#### Scenario: Video newly added by the same pass
- **WHEN** a reconcile pass adds a new video to the channel, and that video has no recorded thumbnail filename
- **THEN** the pass itself does not schedule a thumbnail fetch for that video; its fetch is scheduled in reaction to its `VideoAddedToChannel` event

#### Scenario: Thumbnail fetch already queued
- **WHEN** a reconcile pass finds a video with no recorded thumbnail filename that already has a thumbnail fetch task pending or running
- **THEN** the system does not schedule another thumbnail fetch for it

#### Scenario: Retried thumbnail fetch fails again
- **WHEN** a scheduled thumbnail fetch for a video fails
- **THEN** the video's status and any other recorded field are left unchanged, and the next reconcile pass schedules another fetch, unless the video's download status has meanwhile become Excluded or Errored

#### Scenario: Excluded video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel with no recorded thumbnail filename whose download status is Excluded
- **THEN** the system does not schedule a thumbnail fetch for it, on this or any later pass while it stays Excluded

#### Scenario: Errored video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel with no recorded thumbnail filename whose download status is Errored and that the same pass does not reset for redownload
- **THEN** the system does not schedule a thumbnail fetch for it
