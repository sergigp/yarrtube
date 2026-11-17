# Spec Delta

## ADDED Requirements

### Requirement: Don't Download A Video From The Video Detail
The video detail pane SHALL show a "Don't download" button for a video whose download is Pending, Retrying, Failed or Downloaded. Activating it SHALL ask for confirmation, saying the video will never be downloaded, it can't be undone, and, for a downloaded video, that its files are deleted. Confirming SHALL discard the video (see `video-discard`) and refresh the list, without a page reload; cancelling SHALL change nothing.

#### Scenario: Discard a retrying video
- **WHEN** a user selects a video whose download is Retrying, activates "Don't download" and confirms
- **THEN** the video is discarded and no longer appears in the list

#### Scenario: Confirmation for a downloaded video
- **WHEN** a user selects a downloaded video and activates "Don't download"
- **THEN** the confirmation says the video's files will be deleted and that this can't be undone

#### Scenario: Cancel
- **WHEN** a user activates "Don't download" and cancels the confirmation
- **THEN** no request is sent and the video is shown as before

#### Scenario: Downloading video
- **WHEN** a user selects a video whose download is In Progress
- **THEN** no "Don't download" button is shown

#### Scenario: Discard fails
- **WHEN** a user confirms "Don't download" and the request fails
- **THEN** the application tells the user and the video is shown as before
