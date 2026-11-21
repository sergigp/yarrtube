# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: Don't Download A Video From The Video Detail`
- TO: `### Requirement: Discard A Video From The Video Detail`

## MODIFIED Requirements

### Requirement: Discard A Video From The Video Detail
The video detail pane SHALL show a "Discard" button for a video whose download is Pending, Retrying, Failed or Downloaded. Activating it SHALL ask for confirmation, saying the video won't be downloaded, that it can be restored from its channel's or playlist's menu, and, for a downloaded video, that its files are deleted. Confirming SHALL discard the video (see `video-discard`) and refresh the list, without a page reload; cancelling SHALL change nothing.

#### Scenario: Discard a retrying video
- **WHEN** a user selects a video whose download is Retrying, activates "Discard" and confirms
- **THEN** the video is discarded and no longer appears in the list

#### Scenario: Confirmation for a downloaded video
- **WHEN** a user selects a downloaded video and activates "Discard"
- **THEN** the confirmation says the video's files will be deleted and that it can be restored from its channel's or playlist's menu

#### Scenario: Cancel
- **WHEN** a user activates "Discard" and cancels the confirmation
- **THEN** no request is sent and the video is shown as before

#### Scenario: Downloading video
- **WHEN** a user selects a video whose download is In Progress
- **THEN** no "Discard" button is shown

#### Scenario: Discard fails
- **WHEN** a user confirms "Discard" and the request fails
- **THEN** the application tells the user and the video is shown as before

### Requirement: Video Actions Menu
Every video card on the home, playlist and category views and every row of a playlist or channel detail view's video list, the selected video's row included, SHALL show an always-visible vertical "⋮" control to the right of the video's title. Activating it SHALL open a menu with a "Mark as watched" item and a "Discard…" item, without selecting or navigating to the video. Choosing "Mark as watched" SHALL mark the video watched without confirmation. It SHALL be disabled when the video is already watched or has not finished downloading. When marking fails, the application SHALL tell the user and leave the video as it was. "Discard…" SHALL be offered for a video whose download is Pending, Retrying, Failed or Downloaded, and SHALL ask for the same confirmation as the video detail pane's "Discard" before discarding the video (see `video-discard`); once discarded the video leaves the view without a page reload, and when discarding fails the application SHALL tell the user and leave the video as it was. On a home card whose source is a playlist, the menu SHALL also offer an item labeled `Exclude "<playlist name>" from home`, which excludes that playlist from home without confirmation and refreshes the home view's sections; when it fails, the application SHALL tell the user and leave the playlist as it was. Home cards whose source is a channel and list rows SHALL NOT offer it. The video detail pane SHALL NOT show a "⋮" control; the selected video can also be marked watched from its "Mark watched" button, as described in "Video Detail Watched Control".

#### Scenario: Marking a video watched from a home card
- **WHEN** a user chooses "Mark as watched" from the menu of a "Continue watching" card
- **THEN** the video leaves "Continue watching" and its channel's sidebar badge decreases, without a page reload

#### Scenario: Marking a video watched from a list row
- **WHEN** a user chooses "Mark as watched" from the menu of a row in a channel or playlist video list
- **THEN** the row's thumbnail shows a tick, and the selected video does not change

#### Scenario: Marking the selected video watched from its detail pane
- **WHEN** a user activates "Mark watched" in the video detail pane
- **THEN** the selected video is watched and its row in the list shows a tick

#### Scenario: The selected row leaves the menu to the detail pane
- **WHEN** a video is selected in a playlist or channel detail view
- **THEN** its row in the list shows a "⋮" control like every other row, and the detail pane shows none

#### Scenario: Opening the menu doesn't navigate
- **WHEN** a user opens the menu on a home card or a list row
- **THEN** the application neither navigates nor changes the selected video

#### Scenario: Already watched video
- **WHEN** a user opens the menu of a watched video
- **THEN** "Mark as watched" is shown disabled

#### Scenario: Video not downloaded yet
- **WHEN** a user opens the menu of a video that is pending, downloading or errored
- **THEN** "Mark as watched" is shown disabled

#### Scenario: Excluding a card's playlist from home
- **WHEN** a user chooses `Exclude "Bluey" from home` from the menu of a home card whose source is the playlist "Bluey"
- **THEN** every card sourced from "Bluey" leaves the home view, without a page reload

#### Scenario: Channel cards have no exclude item
- **WHEN** a user opens the menu of a home card whose source is a channel
- **THEN** the menu offers "Mark as watched" and "Discard…" only

#### Scenario: List rows and the detail pane have no exclude item
- **WHEN** a user opens the menu of a row in a playlist's video list
- **THEN** the menu offers "Mark as watched" and "Discard…" only, and the video detail pane offers no menu at all

#### Scenario: Discarding from a home card
- **WHEN** a user chooses "Discard…" from the menu of a "Latest videos" card and confirms
- **THEN** the confirmation said the video's files will be deleted, and the card leaves the home view without a page reload

#### Scenario: Discarding from a list row
- **WHEN** a user chooses "Discard…" from the menu of a Retrying row in a channel's video list and confirms
- **THEN** the row leaves the list and the selected video does not change

#### Scenario: Cancelling a discard from the menu
- **WHEN** a user chooses "Discard…" from a card's menu and cancels the confirmation
- **THEN** no request is sent and the card stays

#### Scenario: Downloading row
- **WHEN** a user opens the menu of a row whose download is In Progress
- **THEN** the menu offers no "Discard…"

### Requirement: Retry From Tasks View
A **Needs attention** card for a task that works on one video SHALL offer **Retry now**, which retries the video. While it is in progress it SHALL be disabled; on success the list SHALL refresh; on failure the card SHALL say the retry failed. A failing download's card SHALL also offer **Discard**, which asks for the same confirmation as the video detail pane's "Discard" before discarding the video (see `video-discard`); on success the list SHALL refresh without the download's task, and on failure the card SHALL say the discard failed. Cards of other per-video tasks SHALL NOT offer **Discard**.

#### Scenario: Retry now
- **WHEN** a user activates **Retry now** on a failing download's card
- **THEN** the application retries that video and refreshes the list

#### Scenario: Action fails
- **WHEN** the retry request fails
- **THEN** the card says the retry failed and its actions can be used again

#### Scenario: Discard a failing download
- **WHEN** a user activates **Discard** on a failing download's card and confirms
- **THEN** the application discards that video and the card leaves the list

#### Scenario: Discard fails
- **WHEN** a user confirms **Discard** on a failing download's card and the request fails
- **THEN** the card says the discard failed and its actions can be used again

#### Scenario: Failing subtitle fetch
- **WHEN** a subtitle fetch is under **Needs attention**
- **THEN** its card offers **Retry now** and no **Discard**

## ADDED Requirements

### Requirement: Restore Discarded Videos From A Source's Menu
The "⋮" menu of a channel's or playlist's page header SHALL offer "Restore discarded videos". It SHALL ask for confirmation, naming the source and saying its discarded videos still in the playlist, or still among the channel's most recent uploads, will be downloaded again, then restore the source's discarded videos (see `video-discard`) and refresh its video list without a page reload. When restoring fails, the application SHALL tell the user.

#### Scenario: Restoring a channel's discarded videos
- **WHEN** a user chooses "Restore discarded videos" from the menu of "@GothamChess", whose discarded video is among its most recent uploads, and confirms
- **THEN** the daemon is asked to restore the channel's discarded videos and the video reappears in its list as pending

#### Scenario: Cancelled
- **WHEN** a user chooses "Restore discarded videos" and cancels the confirmation
- **THEN** nothing is sent to the daemon

#### Scenario: Restore fails
- **WHEN** a user confirms "Restore discarded videos" and the request fails
- **THEN** the application tells the user

#### Scenario: Category menu
- **WHEN** a user opens the "⋮" menu of a category's page header
- **THEN** it offers no "Restore discarded videos"
