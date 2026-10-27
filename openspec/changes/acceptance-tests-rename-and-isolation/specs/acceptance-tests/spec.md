## Purpose

Gives yarrtube an automated acceptance suite that drives the real Docker image through a browser against a fake YouTube (a stand-in Data API and `yt-dlp`): adding playlists and channels, downloading their videos and playing them back, on desktop and on an emulated iPhone, runnable on demand and, unlike the YouTube contract tests, without YouTube.

## ADDED Requirements

### Requirement: Dockerized Test Run
The system SHALL provide a script that builds the release Docker image from the repository's `Dockerfile`, runs a container from it with a fresh, isolated database and videos directory, and waits for `GET /status` to return `200` before running any test, failing with a non-zero exit code and the container's logs if readiness is not reached within a bounded timeout.

#### Scenario: Container becomes ready
- **WHEN** the script starts the built image
- **THEN** it polls `GET /status` until it returns `200`, then proceeds to run the test suite

#### Scenario: Container never becomes ready
- **WHEN** `GET /status` does not return `200` within the timeout
- **THEN** the script exits non-zero, prints the container's logs, and does not attempt to run the test suite

### Requirement: Guaranteed Cleanup
The system SHALL remove the container and any temporary database/videos directories it created after the run, whether the run succeeded, failed, or was interrupted.

#### Scenario: Test run fails
- **WHEN** the Playwright suite reports failures
- **THEN** the script still removes the running container and temporary directories before exiting non-zero

### Requirement: Failure Diagnostics
The system SHALL, when the run fails, preserve the container's logs and the Playwright run's traces/videos as files on disk before cleanup, so they can be inspected or uploaded as CI artifacts.

#### Scenario: A Playwright assertion fails
- **WHEN** a test in the suite fails
- **THEN** the failing test's trace and video are retained on disk, and the container's logs up to that point are saved to a file

### Requirement: Playlist Golden Path
The system SHALL verify, through the browser, that adding a `youtube_linked` playlist results in its video being downloaded and playable, and that deleting the playlist removes it from the UI.

#### Scenario: Add playlist, download, play, delete
- **WHEN** a user adds the fake YouTube's fixture playlist through the Add dialog
- **THEN** the playlist appears in the sidebar, a download task appears in the Tasks view, the video's status progresses to Downloaded, the video appears in the playlist detail view and the home feed with a thumbnail and duration, and it plays back in the browser
- **WHEN** the user then deletes the playlist from the sidebar
- **THEN** it no longer appears in the sidebar or the home feed

### Requirement: Channel Golden Path
The system SHALL verify, through the browser, that adding a `youtube_linked` channel results in its newest videos being downloaded and playable, and that deleting the channel removes it from the UI.

#### Scenario: Add channel, download, play, delete
- **WHEN** a user adds the fake YouTube's fixture channel with a video limit of 1 through the Add dialog
- **THEN** the channel appears in the sidebar, its newest video reaches Downloaded status, and that video plays back in the browser
- **WHEN** the user then deletes the channel from the sidebar
- **THEN** it no longer appears in the sidebar or the home feed

### Requirement: Sync Action Coverage
The system SHALL verify that triggering the sidebar "Sync" action on an already-tracked playlist or channel completes without error.

#### Scenario: Sync an existing playlist
- **WHEN** a user clicks the Sync button on a tracked playlist in the sidebar
- **THEN** the action completes without showing an error

### Requirement: Add-Dialog Error Handling Coverage
The system SHALL verify that the Add dialog surfaces a visible error message when creating a playlist with a path already used by another playlist, when creating a channel with an invalid handle, and when entering a playlist id YouTube does not know.

#### Scenario: Duplicate playlist path
- **WHEN** a user submits the Add dialog for a playlist whose path is already used by another tracked playlist
- **THEN** the dialog shows an error message and the Advanced options section is expanded

#### Scenario: Invalid channel handle
- **WHEN** a user submits the Add dialog for a channel with a handle that does not resolve to a real channel
- **THEN** the dialog shows an error message

#### Scenario: Unknown playlist
- **WHEN** a user enters a well-formed playlist id that YouTube does not know
- **THEN** the notice says the playlist does not exist and Create Playlist stays disabled

### Requirement: Mobile Sidebar Coverage
The system SHALL verify, in the iPhone browser, that the sidebar can be opened via the header's menu control and closed again by that control or by tapping the backdrop, with the control's expanded state reflecting whether the sidebar is open.

#### Scenario: Open and close the mobile sidebar
- **WHEN** a user on a phone taps the menu button
- **THEN** the sidebar becomes visible and the control reports it as expanded, and tapping the control or the backdrop hides the sidebar and the control reports it as collapsed

### Requirement: Announcements Bar Coverage
The system SHALL verify, through the browser against the real image, that the announcements bar displays, links, dismisses, and persists correctly. The GitHub announcements URL SHALL be stubbed with a fixed fixture so the test does not depend on the live file.

#### Scenario: Show, follow link target, dismiss, persist
- **WHEN** the announcements URL is stubbed to return `[{"id":"acceptance-b","text":"Second [docs](https://example.com/docs)"},{"id":"acceptance-a","text":"First"}]` and the user opens `/` in a fresh browser context
- **THEN** a bar shows `Second docs`, where `docs` is a link to `https://example.com/docs` with `target="_blank"`
- **WHEN** the user clicks `Dismiss announcement`
- **THEN** the bar shows `First`
- **WHEN** the user dismisses it and reloads the page
- **THEN** no announcements bar is shown

#### Scenario: Announcements fetch fails
- **WHEN** the announcements URL is stubbed to abort and the user opens `/`
- **THEN** no announcements bar is shown and the header and sidebar render normally

### Requirement: Continuous Integration
The system SHALL run the acceptance suite automatically in CI on every pull request and on every push to `main`, using a `YOUTUBE_API_KEY` provided as a repository secret.

#### Scenario: Pull request opened
- **WHEN** a pull request is opened or updated
- **THEN** the acceptance-tests workflow runs the dockerized suite and reports its result on the pull request

### Requirement: Mobile Browser Projects
The suite SHALL run every spec tagged as mobile in an emulated iPhone browser running WebKit, with touch input and a mobile viewport, and only there. Specs SHALL NOT emulate a phone by resizing the desktop browser. Untagged specs SHALL run in desktop Chromium only.

#### Scenario: A mobile-tagged spec runs on the iPhone
- **WHEN** the full suite runs
- **THEN** each mobile-tagged spec is reported only for the iPhone/WebKit browser

#### Scenario: A desktop spec stays on desktop
- **WHEN** the full suite runs
- **THEN** an untagged spec is reported only for desktop Chromium

### Requirement: Mobile Watch Flow Coverage
The suite SHALL verify on the iPhone browser that tapping a video card on the home page opens that video's source with that video selected, that it plays, and that after recording a position and reloading, playback resumes from it.

#### Scenario: Tap a home card and watch
- **WHEN** a downloaded video is on the home page and the user taps its card on a phone
- **THEN** the source's view opens with that video's title in the detail pane, and the player plays it

#### Scenario: Resume after reload on a phone
- **WHEN** the user pauses halfway through and reloads the page
- **THEN** the player resumes from a position after the start

### Requirement: Mobile Layout Coverage
The suite SHALL verify on the iPhone browser that:
- no view (home, playlist, channel, category, tasks) scrolls horizontally
- the player stays directly below the header while the video list scrolls
- the video details start collapsed and "More" expands them
- the add, edit-settings and delete-confirm dialogs fit inside the viewport with their submit button reachable

#### Scenario: No horizontal overflow
- **WHEN** a user opens any view on a phone
- **THEN** the page is no wider than the viewport

#### Scenario: Sticky player
- **WHEN** a user scrolls a playlist's video list on a phone
- **THEN** the player's top edge stays at the header's bottom edge

#### Scenario: Collapsed video details
- **WHEN** a user opens a video on a phone
- **THEN** its description and "Open on YouTube" link are hidden until the user taps "More"

#### Scenario: Dialogs fit the phone screen
- **WHEN** a user opens the Add playlist dialog (with the folder browser), the Edit settings dialog or the Delete confirmation on a phone
- **THEN** the dialog lies within the viewport and its submit button can be tapped to complete the action

### Requirement: Mobile Navigation Coverage
The suite SHALL verify on the iPhone browser that:
- tapping a source in the open drawer navigates to it and closes the drawer
- Escape closes the drawer
- dismissing the drawer without navigating restores the previous scroll position
- the page behind the open drawer does not scroll
- icon-only controls (Tasks tabs, "Run now", the header's Sync) keep accessible names and work

#### Scenario: Navigate from the drawer
- **WHEN** a user opens the drawer on a phone and taps a tracked playlist
- **THEN** the playlist's view is shown and the drawer is closed

#### Scenario: Escape closes the drawer
- **WHEN** the drawer is open and the user presses Escape
- **THEN** the drawer is closed

#### Scenario: Background stays still
- **WHEN** the drawer is open and the user scrolls over the page behind it
- **THEN** the page's scroll position does not change, and it is unchanged after the drawer closes

#### Scenario: Icon-only controls
- **WHEN** a user on a phone opens the Tasks view and taps the Syncs tab by its accessible name, then a sync's "Run now"
- **THEN** the Syncs tab is selected and the sync runs without error

### Requirement: Category Detail Coverage
The suite SHALL verify that a category's view lists the downloaded videos of its sources as cards and that a card opens its video, selected, in its source's view.

#### Scenario: Play from a category
- **WHEN** a category holds a channel with a downloaded video and the user opens the category and taps that video's card
- **THEN** the channel's view opens with that video's title in the detail pane, and it plays

### Requirement: Playback Speed Coverage
The suite SHALL verify that choosing a speed from the video detail's speed menu changes the player's playback rate and that the menu shows the chosen speed.

#### Scenario: Change speed
- **WHEN** a user chooses 1.5x from the speed menu of a downloaded video
- **THEN** the player's playback rate is 1.5 and the menu shows 1.5x

### Requirement: Deep Link Coverage
The suite SHALL verify that reloading the page on each client route (home, playlist, channel, category, tasks) shows that view, and that opening the route of a source that is not tracked shows an error state rather than a blank page.

#### Scenario: Reload a deep link
- **WHEN** a user reloads the page on `/channels/<handle>`, `/categories/<id>` or `/tasks`
- **THEN** that view is shown

#### Scenario: Unknown source
- **WHEN** a user opens `/playlists/<id>` for a playlist that is not tracked
- **THEN** the view says "Playlist not found."

### Requirement: Source Settings And Task Coverage
The suite SHALL verify that the Tasks view's Downloads tab lists a download while it is running, and that channel avatars load.

#### Scenario: Download listed while running
- **WHEN** a newly added playlist's video is downloading
- **THEN** the Downloads tab lists its download

#### Scenario: Channel avatar loads
- **WHEN** a tracked channel's avatar is shown in the sidebar
- **THEN** the image has loaded with non-zero natural dimensions

### Requirement: Delete During Download Coverage
The suite SHALL verify that deleting a source whose downloads have not finished leaves nothing of it on disk.

#### Scenario: Delete mid-download
- **WHEN** a user deletes a playlist whose video is still downloading
- **THEN** the playlist's folder is removed and does not reappear once the in-flight work ends

### Requirement: Suite Health Checks
Every test SHALL fail on:
- an uncaught page error
- a `console.error` message not on an explicit allowlist
- a failed request for media or avatars
- a 5xx `/api` response
- a 4xx `/api` response the test did not declare as expected

Image assertions SHALL require the image to have loaded, not merely be present. A full passing run SHALL fail if any router `/api` route was never answered with a 2xx status.

#### Scenario: Unexpected console error
- **WHEN** the page logs a `console.error` that is not allowlisted during a test
- **THEN** the test fails naming the message

#### Scenario: Route only hit with errors
- **WHEN** a full passing run hit a route but every response to it was 4xx
- **THEN** the run fails listing that route

#### Scenario: Broken thumbnail
- **WHEN** a video card shows its placeholder because the thumbnail failed to load
- **THEN** the thumbnail assertion fails

### Requirement: Fake YouTube
The suite SHALL run the daemon against a fake YouTube: a fake YouTube Data API and a stand-in `yt-dlp` serving the same fixture catalogue of playlists, a channel and videos from files in the repository, so the suite needs no YouTube API key and makes no request to YouTube. Only the upgrade check's seeding phase, which runs the released image, SHALL use the real YouTube, and it SHALL be skipped when no YouTube API key is available.

#### Scenario: Suite without YouTube
- **WHEN** the suite runs with no YouTube API key and no access to YouTube
- **THEN** every test except the skipped upgrade check runs, and downloads come from the fixture catalogue

#### Scenario: Fixture video downloaded
- **WHEN** a test adds a fixture playlist
- **THEN** its video downloads within seconds, with its thumbnail and subtitles, and plays

### Requirement: Errored Download Coverage
The suite SHALL verify that a video whose download keeps failing has its download listed with the reason the downloader reported while it is retried, and is shown as errored once its retries run out.

#### Scenario: Download fails
- **WHEN** a user adds the fixture playlist holding a video whose download always fails
- **THEN** the Downloads tab lists its download with the failure reason while it is retried, and once the retries run out the video is shown as Errored

### Requirement: Search Coverage
The suite SHALL verify the sidebar's search and the Tasks view's search, which only appear once there are more than 15 sources or tasks.

#### Scenario: Sidebar search
- **WHEN** more than 15 playlists are tracked and a user types part of one's name into the sidebar search
- **THEN** the sidebar lists only the matching playlists

#### Scenario: Tasks search
- **WHEN** a user types part of a playlist's name into the Tasks view's search
- **THEN** only that playlist's tasks are listed

### Requirement: Premiere Coverage
The suite SHALL verify that a playlist's upcoming premiere is not listed or downloaded until it is watchable, while its other videos are.

#### Scenario: Upcoming premiere
- **WHEN** a user adds the fixture playlist holding an upcoming premiere and a regular video
- **THEN** only the regular video is listed and downloaded

### Requirement: Media Delivery Coverage
The suite SHALL verify that the media endpoint answers a byte-range request with a partial response, and that a downloaded video plays in the iPhone/WebKit browser.

#### Scenario: Range request
- **WHEN** the suite requests the video's media URL with `Range: bytes=0-1023`
- **THEN** the response is `206` with a `Content-Range` header and 1024 bytes

### Requirement: Isolated Runs
The suite's run script SHALL give each run its own container, image tag and ports, so that runs started from different worktrees at the same time neither stop each other's containers nor collide on ports, and SHALL keep a failed run's container log with that worktree's test results.

#### Scenario: Two worktrees at once
- **WHEN** the suite is started from two worktrees at the same time
- **THEN** each run starts and removes only its own container, its services listen on ports of their own, and both runs complete

#### Scenario: Ports chosen by the developer
- **WHEN** a developer sets the suite's port variables
- **THEN** the run uses those ports instead of free ones
