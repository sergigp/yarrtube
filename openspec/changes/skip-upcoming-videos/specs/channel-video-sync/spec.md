## ADDED Requirements

### Requirement: Not Yet Watchable Videos Are Not Discovered (Channels)
The system SHALL leave out of a channel's discovered videos any entry that
`yt-dlp` reports as an upcoming premiere or live stream, a stream that is
currently live, or a stream that has ended but is not yet processed. A
left-out entry SHALL be treated as absent from the channel's current
uploads: it is not stored, no download or thumbnail fetch is queued for it,
and the recency positions of the remaining discovered videos SHALL be
contiguous from `0`, most recent first. Once `yt-dlp` no longer reports the
entry that way, the next reconcile SHALL discover it like any other new
upload.

#### Scenario: Channel listing includes an upcoming premiere not yet stored
- **WHEN** a reconcile pass discovers a channel's videos and one entry is an upcoming premiere with no stored record
- **THEN** the system does not store it, does not queue its download or thumbnail fetch, and does not publish a `VideoAddedToChannel` event for it

#### Scenario: Channel listing includes a video that is live now
- **WHEN** a reconcile pass discovers a channel's videos and one entry is currently live, or has ended but is not yet processed
- **THEN** the system ignores it the same way as an upcoming premiere

#### Scenario: Positions skip the ignored entry
- **WHEN** a reconcile pass discovers an upcoming premiere listed first, followed by two regular uploads
- **THEN** the two regular uploads get positions `0` and `1`

#### Scenario: Stored video is listed as upcoming
- **WHEN** a reconcile pass finds that a video already stored for the channel is now listed as upcoming
- **THEN** the system evicts it as for any video no longer among the channel's current uploads

#### Scenario: Premiere becomes watchable
- **WHEN** a reconcile pass runs after a previously ignored premiere has aired and is no longer reported as upcoming, live or post-live
- **THEN** the system stores it with status PENDING and publishes a `VideoAddedToChannel` event, as for any newly seen upload
