# Spec Delta

## ADDED Requirements

### Requirement: Browse Pages Keep Videos On A Failed Refresh
When refreshing a playlist's or category's videos fails after they have loaded, its browse page SHALL keep showing the loaded videos, toolbar and header, and SHALL NOT show an error. It SHALL say the videos failed to load only when none have loaded yet.

#### Scenario: Refresh fails after the grid loaded
- **WHEN** a playlist's videos have loaded and a later refresh of them fails, e.g. while the daemon restarts
- **THEN** the playlist page keeps showing its videos and no "Failed to load videos" message

#### Scenario: First load fails
- **WHEN** a category's videos fail to load before any have loaded
- **THEN** the category page says "Failed to load videos" with the reason

## MODIFIED Requirements

### Requirement: Category View
Each category SHALL have a view at an address of its own with a page header, as described in "Detail View Page Header", and its videos as a grid of the same cards as the home view, each YouTube video once, each card keeping its source's avatar or playlist icon and name, filtered and ordered by the toolbar described in "Video List Toolbar" and "Category Source Filter". Activating a card SHALL open the video within the category, as described in "Category Playback". The header's badge SHALL be a dot in the category's colour, with no tinted background, and its meta line SHALL read "<n> videos · <n> unwatched · from <n> channels and playlists" (naming one kind, e.g. "from 3 channels", when all are alike), the unwatched part only when above 0. The unwatched count SHALL count every downloaded, unwatched video the view lists, including those from playlists whose unwatched count is hidden, so it matches the "Unwatched" chip; until the videos load, the confirmation of "Mark all watched" SHALL use the category's own unwatched count. Its primary action SHALL be "Play newest unwatched"; its actions "Edit category", which opens the category settings dialog, "Mark all watched" (only when the category has sources or videos), which marks every downloaded video of the category watched after the confirmation described in "Confirm Bulk Mark Watched", and "Delete", which deletes the category after the same confirmation as the sidebar row's "Delete" and then navigates to the home view. When the category is known from the categories list, the header SHALL show its name and colour at once, before its videos load, with a placeholder in place of the meta line; the videos SHALL show as placeholders until they load. A category with no sources and no videos SHALL say so and offer a control that opens the category settings dialog. A category with sources but without downloaded videos SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

#### Scenario: Category videos
- **WHEN** a user opens the "Chess" view, whose 69 videos come from 9 channels and playlists, 50 of them unwatched
- **THEN** the view shows a dot in its colour, "Chess", "69 videos · 50 unwatched · from 9 channels and playlists", and the videos as cards, newest first

#### Scenario: Changing the color
- **WHEN** a user chooses "Edit category" on the "Chess" view, chooses "Violet" and saves
- **THEN** "Chess" is saved with the color "#8b5cf6", and the header's dot, the sidebar dot and the home chips of its videos show it

#### Scenario: Renaming
- **WHEN** a user chooses "Edit category" on the "Chess" view, changes the name to "Board games" and saves
- **THEN** the header and the sidebar row show "Board games"

#### Scenario: Header while loading
- **WHEN** a user opens the "Chess" view from the sidebar and its videos are still loading
- **THEN** the header already shows "Chess" with its colour and actions, a placeholder in place of the meta line, and placeholder cards below

#### Scenario: Deleting from the category view
- **WHEN** a user chooses "Delete" from the "Chess" view's header and confirms
- **THEN** the category is deleted and the application navigates to the home view

#### Scenario: Empty category
- **WHEN** a user opens a category with no sources and no videos
- **THEN** the view says it holds no channels or playlists yet and offers a control that opens the category settings dialog

#### Scenario: Unknown category
- **WHEN** a user opens the view of a category that does not exist
- **THEN** the view says the category was not found

#### Scenario: Card opens the video in the category
- **WHEN** a user activates a video card on the "Chess" view
- **THEN** the application stays on the "Chess" view and plays that video, as described in "Category Playback"

#### Scenario: Header tinted
- **WHEN** a user opens the "Chess" view and "Chess" is colored "#0ea5e9"
- **THEN** the header has no tinted background, and its badge is a dot of "#0ea5e9"

#### Scenario: No separate color action
- **WHEN** a user opens the header actions of the "Chess" view
- **THEN** there is no "Change color" action; the colour is changed from "Edit category"

#### Scenario: Mark all watched
- **WHEN** a user chooses "Mark all watched" from a category view's "⋮" menu and confirms
- **THEN** every card shows as watched, and the category's sidebar count disappears promptly

#### Scenario: Unwatched count includes hidden playlists
- **WHEN** a user opens the "Chess" view, whose 10 videos include 3 unwatched from a channel and 2 unwatched from a playlist whose unwatched count is hidden
- **THEN** the meta line reads "10 videos · 5 unwatched · from 2 channels and playlists", the "Unwatched" chip shows 5, and the sidebar badge of "Chess" shows 3

### Requirement: Category Playback
A category view SHALL play its videos like a playlist view. Activating a card on the category view SHALL open that video within the category, at an address that identifies both the category and the video, with the video selected and autoplaying as for a deep-linked video. In this mode the view SHALL keep the category's header, in its watching form, and SHALL show the player and the selected video's info, the same as a channel or playlist view. The category's videos, in the order chosen on its toolbar, SHALL be the queue beside them, as described in "Watching Queue", its header naming that order ("newest first" or "oldest first"). Selecting a video in the queue SHALL play it without leaving the category. Each video SHALL play from, offer the subtitles of, and record its watch progress against the channel or playlist the category lists it from. The video info SHALL link that channel, with its avatar, or that playlist, with the playlist icon. Opening the category without a video, for example from the sidebar or its name in the header, SHALL show the grid.

#### Scenario: Opening a card plays it in the category
- **WHEN** a user activates the card of the video "Rook endings", listed from the playlist "Endgames", on the "Chess" view
- **THEN** the address identifies the "Chess" category and "Rook endings", the player autoplays "Rook endings" from the playlist "Endgames", and the queue beside it shows the videos of "Chess"

#### Scenario: Queue switches across sources
- **WHEN** a user playing "Rook endings" in the "Chess" view selects "Blitz", listed from the channel "Hikaru", in the queue
- **THEN** the player plays "Blitz" from the channel "Hikaru", the view stays on "Chess", and the video info shows Hikaru's avatar linking to the channel

#### Scenario: Progress recorded against the source
- **WHEN** a user watches a video in a category view past the watched threshold
- **THEN** the video is watched in its channel or playlist and in the category, and the badges update as when watched from its source's view

#### Scenario: Back to the grid
- **WHEN** a user playing a video in the "Chess" view activates "Chess" in the sidebar
- **THEN** the view shows the grid of the category's videos

#### Scenario: Queue names the chosen order
- **WHEN** a user who chose "Oldest first" on the "Chess" view plays its oldest video
- **THEN** the queue lists the videos oldest first and its header reads "1 of <n> · oldest first"

### Requirement: Watching Queue
Beside the player in a channel's, playlist's or category's watching layout (below it on phones) the video list SHALL be headed "Up next", with the playing video's position among the listed rows, out of how many are listed, and the list's order ("1 of 32 · playlist order"), and a "Hide watched" switch. With it on, watched videos SHALL leave the list, except the one playing. The switch SHALL be remembered in this browser for every list. In a playlist each row SHALL start with the video's episode number.

#### Scenario: Position in the queue
- **WHEN** a user plays the 5th video of a 32-video playlist
- **THEN** the queue's header reads "Up next" and "5 of 32 · playlist order"

#### Scenario: Hiding watched videos
- **WHEN** a user turns "Hide watched" on while playing a watched video
- **THEN** every other watched video leaves the list, the playing one stays, and the switch is still on after a reload

#### Scenario: Position with watched videos hidden
- **WHEN** a user plays an unwatched video of a 32-video playlist whose first 10 videos are watched, with "Hide watched" on, so 22 rows are listed and the playing one is first
- **THEN** the queue's header reads "1 of 22 · playlist order"
