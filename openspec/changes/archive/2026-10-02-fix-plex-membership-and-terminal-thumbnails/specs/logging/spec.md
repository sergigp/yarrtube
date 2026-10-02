## ADDED Requirements

### Requirement: External Process Diagnostics Reported Through Structured Logs
The system SHALL NOT write the raw diagnostic output (stderr) of the external
download tool (`yt-dlp`) directly to the daemon's log output. When that output
matters for an operation's outcome, the system SHALL report it as part of a
leveled log message carrying the identifying values of the operation (e.g. the
video or channel concerned) as structured fields, consistent with the
"Leveled Log Output" requirement.

#### Scenario: Thumbnail fetch fails with a tool error
- **WHEN** a thumbnail fetch runs and `yt-dlp` exits with an error such as "Video unavailable"
- **THEN** no raw `yt-dlp` stderr line appears in the daemon's log output, and the failure is emitted as one leveled message with the video's id attached as a field and the tool's reason included

#### Scenario: Channel listing fails with a tool error
- **WHEN** listing a tracked channel's videos makes `yt-dlp` exit with an error
- **THEN** no raw `yt-dlp` stderr line appears in the daemon's log output, and the failure is emitted as one leveled message with the channel attached as a field and the tool's reason included

#### Scenario: Tool succeeds
- **WHEN** a thumbnail fetch or channel listing completes successfully
- **THEN** no `yt-dlp` stderr output appears in the daemon's log output
