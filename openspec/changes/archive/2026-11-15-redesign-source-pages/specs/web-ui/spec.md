## ADDED Requirements

### Requirement: Playlist Browse Page
Opening a playlist's view without a selected video SHALL show its browse page: the page header, the toolbar described in "Video List Toolbar", and the playlist's videos as the same cards as the home view, in a grid that is one card per row below the `sm` breakpoint. Activating a card SHALL open the playlist's watching layout with that video selected and autoplaying, at an address identifying the playlist and the video.

#### Scenario: Browsing a playlist
- **WHEN** a user opens the playlist "Chess" from the sidebar
- **THEN** the view shows its header, the watch-state chips and sort, and a card per video whose meta line reads like "Ep 4 · 1y ago", and no player

#### Scenario: Opening a card
- **WHEN** a user activates the card of "The Backward Pawn" on the "Chess" browse page
- **THEN** the address identifies "Chess" and "The Backward Pawn", and the watching layout plays it

### Requirement: Playlist Card Episode
On a playlist's browse page each card's meta line SHALL show "Ep <n>", the video's episode number, and how long ago it was published, in place of the source's avatar and name. A video without an episode number SHALL show only its age.

#### Scenario: Card of an episode
- **WHEN** a user browses a playlist whose episode 4 was published a year ago
- **THEN** that card's meta line reads "Ep 4 · 1y ago"

### Requirement: Playlist Browse Page States
While a playlist's videos load, its browse page SHALL show placeholder cards. A playlist with no recorded videos SHALL say so. A filter that leaves no video SHALL say that no video matches it.

#### Scenario: Empty playlist
- **WHEN** a user opens a playlist with no recorded videos
- **THEN** the view says no videos are recorded for the playlist yet

#### Scenario: Nothing matches
- **WHEN** a user presses "In progress" on a playlist with no started video
- **THEN** the page says no video matches the filter

### Requirement: Play Next
The page header's primary action SHALL be a link playing the video chosen in "Play Next Target": "Play next · Ep <n>" and, from `sm` up, the title on a playlist's browse page; "Play next" and the title on a channel's view, unless that video is already playing; "Play newest unwatched" on a category's browse page. A started video SHALL resume from its saved position. Below the desktop breakpoint the link SHALL span the header's full width.

#### Scenario: Gaps in a playlist
- **WHEN** a playlist's episodes 1, 3 and 5 are watched and episode 2 was started at 4:10
- **THEN** "Play next · Ep 2" opens episode 2, which resumes at 4:10

### Requirement: Play Next Target
"Play next" SHALL play, on a playlist, the first downloaded video in playlist order that is not watched; on a channel or category, the newest downloaded video by publish date that is not watched. A video not downloaded yet (pending, downloading, retrying or failed) SHALL never be its target. When no downloaded video is unwatched the action SHALL NOT be shown.

#### Scenario: Channel
- **WHEN** a user opens a channel whose newest video is watched and whose second newest is not
- **THEN** "Play next" opens the second newest video

#### Scenario: Everything watched
- **WHEN** every video of a playlist is watched
- **THEN** its header offers no "Play next"

#### Scenario: Failed episode
- **WHEN** a playlist's episode 1 failed to download and its episodes 2 and 3 are watched
- **THEN** its header offers no "Play next"

#### Scenario: Channel video still downloading
- **WHEN** a channel's newest video is still downloading and its second newest is downloaded and not watched
- **THEN** "Play next" opens the second newest video"

### Requirement: Video List Toolbar
A playlist's and a category's browse page SHALL show one toolbar line above the grid: the chips "All", "Unwatched", "In progress" and "Watched" with their counts, exactly one pressed, then a sort control showing the order. A playlist SHALL offer "Playlist order" (default), "Newest first" and "Oldest first"; a category "Newest first" (default) and "Oldest first". Videos without a publish date SHALL sort last. Below `sm` the chips SHALL scroll sideways in one row.

#### Scenario: Chip counts
- **WHEN** a playlist has 32 videos, 3 watched and 2 started
- **THEN** the chips read "All 32", "Unwatched 29", "In progress 2" and "Watched 3"

### Requirement: Watch-State Chip Counts
"All" SHALL count every listed video, "Unwatched" the downloaded videos not watched, "In progress" those of them with a saved position (a subset of "Unwatched"), and "Watched" the watched videos. A video not downloaded yet SHALL show only under "All", so the "Unwatched" count agrees with the header's unwatched count.

#### Scenario: Video not downloaded
- **WHEN** a playlist has 3 videos, 2 of them downloaded and not watched and 1 that failed to download
- **THEN** the header reads "Playlist · 3 videos · 2 unwatched", the chips read "All 3" and "Unwatched 2", and the failed video shows under "All" only

### Requirement: Remembered Toolbar
The pressed chip and the order SHALL be remembered per playlist and per category in this browser, and restored when the page is opened again. When the browser's storage can't be read or written, the defaults SHALL apply and changes SHALL last until the page is reloaded.

#### Scenario: Filter and sort remembered
- **WHEN** a user presses "Unwatched" and picks "Newest first" on a playlist, leaves and opens it again
- **THEN** "Unwatched" is pressed, the order is "Newest first", and only its unwatched videos show, newest first

#### Scenario: Storage blocked
- **WHEN** the browser refuses access to its storage and a user opens a playlist
- **THEN** "All" is pressed and the order is "Playlist order"

### Requirement: Category Source Filter
When a category's videos come from more than one channel or playlist, its toolbar SHALL offer, before the sort, a "From: <source>" control whose menu lists "All sources" and each of those sources with its avatar or playlist icon and video count, most videos first. Choosing one SHALL show only its videos, and the chips SHALL count only them. The choice SHALL be remembered per category; a remembered source no longer in the category SHALL fall back to "All sources".

#### Scenario: Filtering by source
- **WHEN** a user picks "Eric Rosen" under "From" on the "Chess" view
- **THEN** the button reads "From: Eric Rosen", only Eric Rosen's videos show, and the chips count only them

### Requirement: Watching Queue
Beside the player in a channel's, playlist's or category's watching layout (below it on phones) the video list SHALL be headed "Up next", with the playing video's position and the list's order ("1 of 32 · playlist order"), and a "Hide watched" switch. With it on, watched videos SHALL leave the list, except the one playing. The switch SHALL be remembered in this browser for every list. In a playlist each row SHALL start with the video's episode number.

#### Scenario: Position in the queue
- **WHEN** a user plays the 5th video of a 32-video playlist
- **THEN** the queue's header reads "Up next" and "5 of 32 · playlist order"

#### Scenario: Hiding watched videos
- **WHEN** a user turns "Hide watched" on while playing a watched video
- **THEN** every other watched video leaves the list, the playing one stays, and the switch is still on after a reload

## MODIFIED Requirements

### Requirement: Detail View Page Header
A playlist, channel or category view SHALL open with a page header showing a badge (a channel's avatar, a playlist icon, or a dot in a category's colour), the name, and one meta line below it. For a playlist or channel the meta line SHALL read "<Playlist|Channel> · <n> videos · <n> unwatched · synced <relative time>", the unwatched part only when above 0 (also for a playlist whose count is hidden) and the sync part only once a sync time is known; a playlist's browse page SHALL also show, after it, a chip for each category the playlist belongs to while categories are on, linking to that category. Until the videos load the meta line SHALL show a placeholder. The header SHALL then show the page's primary action, as described in "Play Next", and its actions: for channels and playlists a sync action, a link labeled "Open channel on YouTube" or "Open playlist on YouTube" opening its youtube.com page in a new tab, "Mark all watched", for playlists "Exclude from home" or "Include in home" (changing that setting without confirmation) and the hide or show unwatched count action, "Edit settings" opening the edit channel or playlist dialog, the AI categorization actions while categories are on, and last delete. At or above the desktop breakpoint, sync, the YouTube link and a category's "Edit category" SHALL be round icon-only buttons whose accessible name and tooltip are the action's label, followed by a vertical "⋮" control, named "More actions for <name>", whose menu offers every other action, with delete set apart and styled as destructive. Below the desktop breakpoint the header SHALL stack: the badge, name and meta line with only the "⋮" control beside them, whose menu offers every action, and the primary action below at full width. In the watching layout the header SHALL be smaller, without the primary action on a playlist or category, and a playlist's or category's name SHALL link back to its browse page. Deleting SHALL require confirmation and, once confirmed, SHALL return the user to the home view. When syncing or changing the home setting fails, the application SHALL tell the user and leave the entry as it was.

#### Scenario: Playlist page header
- **WHEN** a user opens a playlist synced 14 days ago with 32 videos, 29 unwatched
- **THEN** the header shows a playlist icon, its name, "Playlist · 32 videos · 29 unwatched · synced 14d ago" and "Play next"

#### Scenario: Header actions on a desktop viewport
- **WHEN** a user views a playlist's page at or above the desktop breakpoint
- **THEN** "Sync" and "Open playlist on YouTube" show as icon buttons with tooltips, and the "⋮" menu offers "Mark all watched", "Exclude from home", "Hide unwatched count", "Edit settings" and, set apart, "Delete"

#### Scenario: Page header on a narrow viewport
- **WHEN** a user views a playlist's page below the desktop breakpoint
- **THEN** only the "⋮" control is beside the name, its menu offers sync, the YouTube link, mark all watched, the home and unwatched count actions, "Edit settings" and, set apart, delete, and "Play next" spans the width below

#### Scenario: Name leads back while watching
- **WHEN** a user watching a video of the playlist "Chess" activates "Chess" in the header
- **THEN** the playlist's browse page shows

#### Scenario: Opening the source on YouTube
- **WHEN** a user activates "Open channel on YouTube" in the page header of the channel `@somechannel`
- **THEN** a new tab opens to `https://www.youtube.com/@somechannel`

#### Scenario: Deleting from the page header
- **WHEN** a user chooses delete from the "⋮" menu and confirms
- **THEN** the application deletes that channel, playlist or category and navigates to the home view

#### Scenario: Excluding a playlist from home from its page header
- **WHEN** a user chooses "Exclude from home" from a playlist's page header
- **THEN** the playlist's videos leave the home view, and the menu now offers "Include in home"

#### Scenario: Channel page header
- **WHEN** a user opens a channel's view
- **THEN** the page header shows the channel's avatar, its name, "Channel · <n> videos" and "Play next", Sync and "Open channel on YouTube" as icon buttons on desktop, and "Mark all watched", "Edit settings" and delete behind "⋮"

#### Scenario: Mark all watched is not a header button on desktop
- **WHEN** a user views a channel's, playlist's or category's view at or above the desktop breakpoint
- **THEN** no icon button in the header marks the entry watched; "Mark all watched" is reached only through the "⋮" menu

#### Scenario: Page header actions on a narrow viewport
- **WHEN** a user views a channel's view below the desktop breakpoint
- **THEN** the header shows only the "⋮" control beside the name, and its menu offers sync, "Open channel on YouTube", "Mark all watched", "Edit settings" and, set apart, delete

#### Scenario: Playlist summary includes its unwatched count
- **WHEN** a user opens the view of a playlist with 3 videos, 2 of them downloaded and unwatched
- **THEN** the meta line starts "Playlist · 3 videos · 2 unwatched", whether or not the playlist's count is hidden

#### Scenario: Marking a playlist watched from its view
- **WHEN** a user chooses "Mark all watched" from a playlist's "⋮" menu and confirms
- **THEN** the "Watched" chip counts every downloaded video, and the playlist's sidebar badge disappears

#### Scenario: Editing a playlist's settings from its page header
- **WHEN** a user chooses "Edit settings" from a playlist's "⋮" menu
- **THEN** the edit playlist dialog opens for that playlist

#### Scenario: Summary omits unwatched count when nothing is unwatched
- **WHEN** a view's channel or playlist has no unwatched videos
- **THEN** the meta line shows no unwatched part

#### Scenario: Syncing from the page header
- **WHEN** a user activates the sync action in a view's page header
- **THEN** the application triggers a reconcile for that channel or playlist and shows it as in progress until it completes

#### Scenario: Including a playlist in home from its page header
- **WHEN** a user chooses "Include in home" from the "⋮" menu of a playlist excluded from home
- **THEN** the playlist's videos appear in the home view's sections they qualify for, and the menu now offers "Exclude from home"

### Requirement: Category View
Each category SHALL have a view at an address of its own with a page header, as described in "Detail View Page Header", and its videos as a grid of the same cards as the home view, each YouTube video once, each card keeping its source's avatar or playlist icon and name, filtered and ordered by the toolbar described in "Video List Toolbar" and "Category Source Filter". Activating a card SHALL open the video within the category, as described in "Category Playback". The header's badge SHALL be a dot in the category's colour, with no tinted background, and its meta line SHALL read "<n> videos · <n> unwatched · from <n> channels and playlists" (naming one kind, e.g. "from 3 channels", when all are alike), the unwatched part only when above 0. Its primary action SHALL be "Play newest unwatched"; its actions "Edit category", which opens the category settings dialog, "Mark all watched" (only when the category has sources or videos), which marks every downloaded video of the category watched after the confirmation described in "Confirm Bulk Mark Watched", and "Delete", which deletes the category after the same confirmation as the sidebar row's "Delete" and then navigates to the home view. When the category is known from the categories list, the header SHALL show its name and colour at once, before its videos load, with a placeholder in place of the meta line; the videos SHALL show as placeholders until they load. A category with no sources and no videos SHALL say so and offer a control that opens the category settings dialog. A category with sources but without downloaded videos SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

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

### Requirement: Category Playback
A category view SHALL play its videos like a playlist view. Activating a card on the category view SHALL open that video within the category, at an address that identifies both the category and the video, with the video selected and autoplaying as for a deep-linked video. In this mode the view SHALL keep the category's header, in its watching form, and SHALL show the player and the selected video's info, the same as a channel or playlist view. The category's videos, in the order chosen on its toolbar, SHALL be the queue beside them, as described in "Watching Queue". Selecting a video in the queue SHALL play it without leaving the category. Each video SHALL play from, offer the subtitles of, and record its watch progress against the channel or playlist the category lists it from. The video info SHALL link that channel, with its avatar, or that playlist, with the playlist icon. Opening the category without a video, for example from the sidebar or its name in the header, SHALL show the grid.

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

### Requirement: Confirm Bulk Mark Watched
Marking a channel, playlist or category watched, from its page header or its sidebar row menu, SHALL always first ask the user to confirm, whatever its number of unwatched videos, since it can't be undone. The confirmation SHALL state how many videos will be marked, counting them also when the count is hidden ("Mark 23 videos in "Foo" watched?", "Mark 1 video in "Foo" watched?", or "Mark everything in "Foo" watched?" when none is known unwatched), and that this cannot be undone in bulk, with a destructive-styled confirm action. Cancelling SHALL mark nothing.

#### Scenario: Confirming marks the entry watched
- **WHEN** a user chooses "Mark all watched" for the channel "Foo", which has 23 unwatched videos
- **THEN** a confirmation asks to mark 23 videos in "Foo" watched and says this cannot be undone in bulk
- **AND** confirming marks the channel watched and closes the confirmation

#### Scenario: One unwatched video still asks
- **WHEN** a user chooses "Mark all watched" for a playlist with exactly 1 unwatched video
- **THEN** a confirmation asks to mark 1 video watched before anything is marked

#### Scenario: Cancelling marks nothing
- **WHEN** a user chooses "Mark all watched" for a playlist with 5 unwatched videos and cancels the confirmation
- **THEN** no video is marked watched and the playlist's badge still shows 5

#### Scenario: Hidden count still confirms
- **WHEN** a user chooses "Mark all watched" from the menu of a playlist whose unwatched count is hidden and which has 4 unwatched videos
- **THEN** a confirmation asks to mark 4 videos watched

#### Scenario: Marking fails
- **WHEN** a user confirms marking an entry watched and the request fails
- **THEN** the confirmation stays open, shows the error, and no video is marked watched

#### Scenario: Marking a category asks for confirmation
- **WHEN** a user chooses "Mark all watched" from the header of a category with 3 unwatched videos
- **THEN** a confirmation asks to mark 3 videos in the category watched before anything is marked

#### Scenario: One unwatched video needs no confirmation
- **WHEN** a user chooses "Mark all watched" for a channel with exactly 1 unwatched video
- **THEN** a confirmation still asks to mark 1 video in the channel watched before anything is marked

#### Scenario: Nothing unwatched needs no confirmation
- **WHEN** a user chooses "Mark all watched" for a playlist with no unwatched videos
- **THEN** a confirmation still asks to mark everything in the playlist watched before anything is marked

### Requirement: Video Detail Meta Line
The selected video's info SHALL show a meta line directly below the title, reading `<source> · Watched · Published <date> · Episode <n>`:
- The source SHALL be, in a channel or category view, the channel's avatar and name, or the playlist icon and name, linking to that channel or playlist; in a playlist view, the video's channel name, as text.
- "Watched" SHALL appear only when the video is watched.
- The publish time SHALL be shown as a date, without a time of day.
- The episode SHALL appear only in a playlist view, for a video with an episode number.
- Any part whose value is absent SHALL be omitted along with its separator. When every part is absent, the line SHALL not be shown.

#### Scenario: Video in a playlist view
- **WHEN** a user selects episode 1 of a playlist, published on Mar 6, 2019 by "Hanging Pawns"
- **THEN** the meta line reads "Hanging Pawns · Published Mar 6, 2019 · Episode 1"

#### Scenario: Video in a channel view
- **WHEN** a user selects a video in a channel view
- **THEN** the meta line starts with the channel's avatar and name, linking to the channel

#### Scenario: Watched video
- **WHEN** a user selects a watched video in a playlist view
- **THEN** "Watched" follows the channel name on the meta line

#### Scenario: Video without metadata
- **WHEN** a user selects a pending video in a playlist view that has no channel name, no publish date and no episode number
- **THEN** no meta line is shown

#### Scenario: Downloaded video in a playlist view
- **WHEN** a user selects a downloaded, unwatched video with generated metadata in a playlist view
- **THEN** the meta line shows the video's channel name, "Published" followed by its publish date, and its episode, and no sync time

#### Scenario: Downloaded video in a channel view
- **WHEN** a user selects a downloaded, unwatched video with generated metadata in a channel view
- **THEN** the meta line shows the channel's avatar and name, linked, and "Published" with its publish date, and no sync time

#### Scenario: Sync time hidden on a mobile viewport
- **WHEN** a user on any viewport selects a synced video
- **THEN** the meta line shows no "Synced" part; the source's last sync is on the page header instead

#### Scenario: Meta line on a collapsed mobile pane
- **WHEN** a user on a mobile-width viewport opens a watching layout
- **THEN** the meta line is visible below the title

#### Scenario: Video without metadata or sync time
- **WHEN** a user selects, in a playlist view, a pending video that has no channel name, publish date or episode number
- **THEN** no meta line is shown

### Requirement: Video Detail AI Summary
The selected video's info SHALL show the video's summary, when the listing reports one, in full below the title row, in a panel labeled "AI summary", at every viewport size; at or above the desktop breakpoint the chapter list described in "Chapter List Beside The Summary" SHALL sit beside it. Line breaks in the summary SHALL be kept. When the listing reports none, the info SHALL show neither the text nor the label.

#### Scenario: Video with a summary
- **WHEN** a user selects a video in a playlist, channel or category view whose listing entry has a summary
- **THEN** the info shows "AI summary" and the whole summary below the title row, with no control to expand it

#### Scenario: Video without a summary
- **WHEN** a user selects a video whose listing entry has no summary
- **THEN** the info shows no "AI summary" label and no placeholder

### Requirement: Video Detail Watched Control
The selected video's info SHALL group its own actions right of the title (below it when they don't fit): a "Mark watched" button with a tick and its label while the video is downloaded and unwatched, which marks the video watched without confirmation, then the playback speed control, then the link to the video on YouTube. The button SHALL NOT be shown once the video is watched or while it has not finished downloading; a watched video is shown as such on the meta line, as described in "Video Detail Meta Line".

#### Scenario: Marking the selected video watched
- **WHEN** a user activates "Mark watched" in the info of a downloaded, unwatched video
- **THEN** the selected video is watched, its row in the queue shows a tick, the button disappears and the meta line shows "Watched"

#### Scenario: Video not downloaded yet
- **WHEN** the selected video is pending, downloading or errored
- **THEN** the info shows no "Mark watched" button

#### Scenario: Marking fails
- **WHEN** a user activates "Mark watched" and the request fails
- **THEN** the application tells the user and the video stays unwatched

#### Scenario: Watched video
- **WHEN** a user selects a watched video
- **THEN** the info shows no "Mark watched" button and its meta line shows "Watched"

#### Scenario: Rewatching resets the control
- **WHEN** a user replays a watched video past 10% of its duration
- **THEN** the video becomes unwatched and the "Mark watched" button shows again, without a page reload

#### Scenario: Visible on a mobile viewport
- **WHEN** a user selects a downloaded, unwatched video below the desktop breakpoint
- **THEN** the "Mark watched" button, with its tick and label, shows below the title, reachable without opening any menu

### Requirement: Category Tints In Dark Mode
In the dark theme, every place that fills a chip with the soft tint of a category's hue (home card chips, the chosen chips of the "Categories" field) SHALL instead use a dark tint of that hue, and every chip name drawn in the dark shade of that hue SHALL instead use a light shade of it. For every palette color, the light shade on the dark tint SHALL have a contrast ratio of at least 4.5:1, and the dark tint SHALL keep the category's hue.

#### Scenario: Home card chip in dark mode
- **WHEN** a user in dark mode views a home card for a video in the "Chess" category whose color is Sky
- **THEN** the "Chess" chip is filled with a dark tint of sky blue and its name is in a light shade of sky blue

#### Scenario: Category header in dark mode
- **WHEN** a user in dark mode opens the "Chess" category view
- **THEN** its header has no tinted background; only its colour dot shows the category's hue

## REMOVED Requirements

### Requirement: Rename Category In Place
**Reason**: The category header no longer holds a rename control; the name is edited in the category settings dialog, reached from "Edit category", like every other category setting.
**Migration**: Rename a category from "Edit category" (or the sidebar row's settings): change "Name" and save.
