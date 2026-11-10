## MODIFIED Requirements

### Requirement: Reconcile Passes Sync Stored Videos
During each playlist or channel reconcile pass, the system SHALL schedule the video sync of every video stored for the playlist or channel once its membership is up to date, including the videos the same pass newly added. Each sync SHALL run as its own task, so a sync that fails for one video SHALL NOT affect the pass or the other videos' syncs. A video whose sync is already pending SHALL NOT get a second one.

#### Scenario: Stored video missing its subtitles
- **WHEN** a reconcile pass runs for a channel with a Downloaded video whose subtitles are Missing, last checked more than 24 hours ago, and no subtitle fetch is pending
- **THEN** a video sync of that video is scheduled, and once it runs a subtitle fetch for that video is scheduled

#### Scenario: Video reset for redownload
- **WHEN** a reconcile pass runs for a channel with a Downloaded video whose file is missing
- **THEN** a video sync of that video is scheduled, and once it runs the video is reset to Pending and a thumbnail fetch and a download of it are scheduled

#### Scenario: Video newly added by the same pass
- **WHEN** a reconcile pass adds a new video to a playlist
- **THEN** one video sync of it is pending, even though its addition also asks for one

#### Scenario: One video fails to sync
- **WHEN** a reconcile pass schedules the syncs of two videos and the first sync fails
- **THEN** the second video's sync still runs, and the failed one is retried like any other failing task

### Requirement: Concurrent Syncs Of One Video Are Serialized
Two video syncs of the same video SHALL NOT run at the same time, nor SHALL a video sync run at the same time as any other task operating on that video (see `task-scheduling`'s "Per-Video Mutual Exclusion"). A sync that becomes eligible while another task of that video runs SHALL start once that task finishes, and decide from the state it left. Syncs of different videos SHALL NOT wait for each other.

#### Scenario: Pass and event sync overlap
- **WHEN** a reconcile pass schedules a video's sync while a video sync scheduled by its download finishing is running for the same video
- **THEN** the second sync starts only once the first finished, and decides from the state the first left

#### Scenario: Sync waits for a running subtitle fetch
- **WHEN** a video's sync becomes eligible while that video's subtitle fetch is running
- **THEN** the sync starts only once the subtitle fetch finished
