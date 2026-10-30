## ADDED Requirements

### Requirement: Video Card Presentation
Video cards on the home view and on a category's view SHALL be drawn without a frame and without a background: the thumbnail and the text below it sit directly on the page background, with no border and no padding around the card. Hovering a card with a pointer SHALL show a soft background behind the whole card and slightly enlarge the thumbnail within its rounded bounds. Below the title, a card SHALL show a meta line:
- **Channel video:** the channel's avatar beside the title, and on the meta line the channel's name followed by " · " and how long ago the video was published.
- **Playlist video:** a placeholder playlist icon in the avatar's place, and on the meta line the playlist's name followed by " · " and how long ago the video was published.
- **No publish date:** the meta line shows only the source's name.

The relative date SHALL read as a short age ("3h ago", "2d ago", "5w ago", "4mo ago", "2y ago").

Activating a card anywhere SHALL open its video, except on the card's own controls: the channel's avatar and the playlist icon lead to the source's view, as does the source's name on the meta line, and the actions menu control opens the menu. Category chips are labels, not controls: activating one opens the video like the rest of the card. Each card SHALL offer a single link to its video, so keyboard users move through cards one stop per video plus the source and menu controls.

#### Scenario: Frameless card
- **WHEN** a user views the home view
- **THEN** no video card has a border, padding or background colour of its own

#### Scenario: Hovering a card
- **WHEN** a user hovers a video card with a pointer
- **THEN** a soft background appears behind the card and its thumbnail is slightly enlarged without overflowing its rounded corners

#### Scenario: Channel card meta line
- **WHEN** a home card shows a video of the channel "BettaTech" published 2 days ago
- **THEN** its meta line reads "BettaTech · 2d ago" and the channel's avatar is shown beside the title

#### Scenario: Playlist card meta line
- **WHEN** a home card shows a video of the playlist "Chess Middlegames" published 5 weeks ago
- **THEN** a playlist icon is shown beside the title and its meta line reads "Chess Middlegames · 5w ago"

#### Scenario: Unknown publish date
- **WHEN** a home card shows a channel video with no publish date
- **THEN** its meta line reads only the channel's name

#### Scenario: Activating the card's empty space
- **WHEN** a user clicks a card's padding or the empty end of its meta line
- **THEN** the application opens that card's video

#### Scenario: Activating the playlist icon
- **WHEN** a user activates the playlist icon on a card of the playlist "Chess Middlegames"
- **THEN** the application opens the "Chess Middlegames" view, not the video

### Requirement: Home Card Category Chips
On the home view's "Continue watching", "Quick watches" and "Latest videos" sections, each video card SHALL show the video's categories, in the order the video lists them, as small text chips below its meta line. Each chip SHALL show the category's name in a dark shade of the category's hue, over a soft tint of that hue. A card SHALL show at most 2 chips, followed by a "+N" chip when the video has more categories. A video in no category SHALL show no chip row. Cards in "More about <category>" sections and on a category's view SHALL NOT show chips.

#### Scenario: One category
- **WHEN** a video under "Latest videos" belongs to "Chess"
- **THEN** its card shows one chip reading "Chess", tinted with Chess's hue

#### Scenario: Many categories
- **WHEN** a video under "Continue watching" belongs to "AI & Code", "Chess" and "Science"
- **THEN** its card shows the chips "AI & Code", "Chess" and "+1"

#### Scenario: No category
- **WHEN** a video under "Quick watches" belongs to no category
- **THEN** its card shows no chip row

#### Scenario: Category sections show no chips
- **WHEN** a user views the "More about Chess" section or the "Chess" view
- **THEN** no card in it shows a category chip

### Requirement: Home Shelves
At or above the small breakpoint, "Continue watching", each category section and "Quick watches" SHALL each show their videos in a single row whose number of columns follows the width available to the content area, not the viewport. A section SHALL show only as many videos as fit in that row and SHALL never leave a partial second row. "Latest videos" SHALL keep a multi-row grid whose rows are always complete except possibly the last. Below the small breakpoint, those same sections SHALL be horizontal rows the user can swipe, each card about three quarters of the content width so that the next card peeks in, snapping card by card.

#### Scenario: Wide desktop
- **WHEN** a category section has 6 videos and the content area fits 5 columns
- **THEN** the section shows 5 videos in one row and no second row

#### Scenario: Collapsed sidebar or narrower content
- **WHEN** the content area narrows so that only 3 columns fit
- **THEN** each shelf section shows 3 videos in one row

#### Scenario: Swiping a shelf on a phone
- **WHEN** a user on a phone-width viewport views "Continue watching" with 4 videos
- **THEN** the section shows one card at about three quarters of the width with the next card peeking in, and swiping horizontally brings the next card into place

### Requirement: Mobile Latest Videos Feed
Below the small breakpoint, "Latest videos" SHALL be a single-column list of full-width cards, each with its full-width thumbnail and its title wrapping on up to 2 lines.

#### Scenario: Latest videos on a phone
- **WHEN** a user on a phone-width viewport scrolls to "Latest videos"
- **THEN** its videos are listed one per row, each thumbnail spanning the content width

### Requirement: Touch-Friendly Video Menu
On touch devices, each video card's and video row's actions menu control SHALL have a hit area of at least 44 by 44 pixels.

#### Scenario: Tapping a card's menu on a phone
- **WHEN** a user on a touch device taps near a card's "⋮" control
- **THEN** the video actions menu opens, from a hit area at least 44 by 44 pixels

### Requirement: Loading Placeholders
While a view's videos are loading, the application SHALL show placeholders shaped like the content they stand for (thumbnail blocks with title and meta lines, or list rows), animated subtly, instead of "Loading…" text. The loading region SHALL be marked busy for assistive technology and SHALL be announced as loading. This applies to the home view, a category's view, and a channel's or playlist's video list. Placeholders SHALL take the space of the content they stand for, so the layout does not shift when the content arrives. When the motion-reduction preference is set, placeholders SHALL NOT animate.

#### Scenario: Home while loading
- **WHEN** a user opens the home view and its videos are still loading
- **THEN** the view shows placeholder sections of placeholder cards, marked busy, and no section heading such as "Latest videos"

#### Scenario: Home after loading
- **WHEN** the home view's videos arrive
- **THEN** the placeholders are replaced by the real sections and nothing is marked busy

#### Scenario: Channel video list while loading
- **WHEN** a user opens a channel's view and its videos are still loading
- **THEN** the video list shows placeholder rows, marked busy, instead of "Loading videos…"

### Requirement: Global Search
The header SHALL offer a search control named "Search" that opens a search dialog. The dialog SHALL also open with ⌘K on macOS and Ctrl+K elsewhere, from anywhere in the application except while another dialog is open. As the user types, the dialog SHALL show results grouped, in this order:
- **Categories**: names containing the text, ignoring case.
- **Channels**: names containing the text, ignoring case.
- **Playlists**: names containing the text, ignoring case.
- **Videos**: the matches the daemon returns for the text, as described in the `video-search` capability, each with its thumbnail, title and source name.

Name matches SHALL appear as the user types. Video results SHALL be requested only once the text has at least 2 characters and the user has paused typing briefly. A group with no matches SHALL be hidden. When no group has matches, the dialog SHALL say nothing matches the text. While videos are being searched, the Videos group SHALL show that it is searching. Results SHALL be navigable with the arrow keys and chosen with Enter or a click. Choosing a category, channel or playlist SHALL navigate to its view; choosing a video SHALL open it in its source's view. Choosing a result or pressing Escape SHALL close the dialog. On phone-width viewports the search control SHALL be an icon button in the header, and the dialog SHALL cover the screen.

#### Scenario: Opening with the keyboard
- **WHEN** a user presses ⌘K on macOS
- **THEN** the search dialog opens with its field focused

#### Scenario: Grouped matches
- **WHEN** a user types "chess" and the category "Chess", the channel "ChessCoach Andras" and videos titled "Chess Tactics: Spot the Trigger" exist
- **THEN** the dialog shows "Chess" under Categories, "ChessCoach Andras" under Channels and the matching videos under Videos, and no Playlists group

#### Scenario: Short text searches names only
- **WHEN** a user types a single character
- **THEN** the dialog shows matching categories, channels and playlists and requests no video search

#### Scenario: Choosing a video
- **WHEN** a user chooses a video result from the channel "BettaTech"
- **THEN** the dialog closes and the application opens that video in the "BettaTech" channel view

#### Scenario: Nothing matches
- **WHEN** a user types "zzz" and nothing matches
- **THEN** the dialog says nothing matches "zzz"

#### Scenario: Video search fails
- **WHEN** the video search request fails
- **THEN** the Videos group says videos could not be searched, and the name matches are still shown

### Requirement: Sidebar Count Style
The unwatched counts on sidebar rows and on folded section headings SHALL be shown as muted, small, tabular numerals at the row's trailing edge, without a filled background, and in the full foreground colour on the active row.

#### Scenario: Count on a channel row
- **WHEN** a channel row shows an unwatched count of 3
- **THEN** the count "3" is shown as a muted numeral without a filled pill behind it

### Requirement: Compact Mobile Announcement Bar
Below the small breakpoint, the announcement bar SHALL take a single line: the announcement's text truncated to one line, followed by its links, with the dismiss control at the trailing edge.

#### Scenario: Long announcement on a phone
- **WHEN** a user on a phone-width viewport sees an announcement whose text is longer than one line
- **THEN** the bar shows a single line with the text truncated, its links and the dismiss control

### Requirement: Video List Row Date
Each row of a channel's or playlist's video list, and of a category's playback list, SHALL show below its title how long ago the video was published, in the same short form as the video cards, when its publish date is known.

#### Scenario: Row with a publish date
- **WHEN** a channel's video list includes a video published 3 days ago
- **THEN** its row shows "3d ago" below the title

#### Scenario: Row without a publish date
- **WHEN** a video in the list has no publish date
- **THEN** its row shows no date

### Requirement: Tasks View Title
The tasks view SHALL open with a page heading "Tasks", in the same heading style and alignment as the other views' page headers.

#### Scenario: Opening the tasks view
- **WHEN** a user opens the tasks view
- **THEN** a "Tasks" heading is shown above the tab bar, aligned with the left edge of the content area

## MODIFIED Requirements

### Requirement: No Category UI While Categories Are Off
While `categories_enabled` is off, or the preferences are still loading, the application SHALL NOT show the sidebar Categories section, the Categories field in the add and edit channel and playlist dialogs, home category sections or home card category chips, and the header search SHALL NOT list a Categories group. When the preferences cannot be loaded, the application SHALL behave as with the defaults.

#### Scenario: Sidebar without categories
- **WHEN** categories are off and "Chess" exists
- **THEN** the sidebar shows no Categories section and no "Add category" control

#### Scenario: Dialogs without categories
- **WHEN** categories are off and a user opens the add channel dialog
- **THEN** the dialog shows no Categories field, and saving it changes no category

#### Scenario: Home without categories
- **WHEN** categories are off and a home video belongs to "Chess"
- **THEN** home shows no "More about Chess" section and the video's card shows no category chip

#### Scenario: Search without categories
- **WHEN** categories are off, "Chess" exists and a user types "chess" in the header search
- **THEN** the search shows no Categories group

#### Scenario: Preferences fail to load
- **WHEN** the preferences request fails
- **THEN** the sidebar shows the Categories section as with the defaults

### Requirement: Persistent Header
At or above the small breakpoint, the application header SHALL remain fixed in place (visible, non-scrolling) as the user scrolls any view. Below the small breakpoint, the header SHALL be 56 pixels tall and SHALL slide out of view while the user scrolls the document down, and slide back as soon as the user scrolls up. It SHALL stay visible while the document is at its top, while the mobile sidebar is open, and while a video player is pinned below it. The header's logo SHALL occupy its final size before the logo image loads, so the header's contents do not shift when it appears.

#### Scenario: Scrolling a view with a tall content area
- **WHEN** a user on a desktop-width viewport scrolls a view whose content exceeds the visible viewport height
- **THEN** the header remains visible in place and does not move with the scrolled content

#### Scenario: Scrolling down on a phone
- **WHEN** a user on a phone-width viewport scrolls the home view down
- **THEN** the header slides out of view

#### Scenario: Scrolling back up on a phone
- **WHEN** a user on a phone-width viewport whose header is hidden scrolls up
- **THEN** the header slides back into view straight away

#### Scenario: Pinned player on a phone
- **WHEN** a user on a phone-width viewport scrolls a channel view whose player is pinned below the header
- **THEN** the header stays visible

#### Scenario: Scrolling on a mobile viewport as the browser's UI chrome shows or hides
- **WHEN** a user on a mobile browser scrolls a view while the browser's own toolbar collapses or expands
- **THEN** the header hides and returns only with the user's scroll direction, and is not displaced by the toolbar's movement

#### Scenario: Logo loading
- **WHEN** the application loads and the logo image has not arrived yet
- **THEN** the space for the logo is already reserved, and the header's other contents do not move when it arrives

### Requirement: Home Page Layout
The home view SHALL display its sections in this order: "Continue watching", up to 3 category sections, "Quick watches" and "Latest videos", loaded together in one request. A video SHALL appear in at most one section. "Latest videos" SHALL display up to 18 recently synced videos. "Continue watching" SHALL display up to 6 videos the user has started and not finished recently. "Quick watches" SHALL display up to 6 short unwatched videos. Each category section SHALL display up to 6 videos, as described in "Home Category Sections". "Continue watching", the category sections and "Quick watches" SHALL be laid out as described in "Home Shelves"; "Latest videos" as a grid, and below the small breakpoint as described in "Mobile Latest Videos Feed". Cards SHALL be presented as described in "Video Card Presentation". "Latest videos" SHALL be hidden when it has no videos while another section has videos; when no section has videos, "Latest videos" SHALL be shown saying no videos have been synced yet. "Continue watching", the category sections and "Quick watches" SHALL be hidden when they have no videos or when they cannot be loaded. While the home videos are loading, the view SHALL show placeholders as described in "Loading Placeholders" and no section heading. Section headings SHALL be set in the heading typeface, larger and bolder than card titles.

#### Scenario: Home view with recent videos
- **WHEN** one or more videos have been synced
- **THEN** the home view renders them under "Latest videos" as a grid of thumbnail-and-title cards

#### Scenario: Channel video card
- **WHEN** a video in any home section comes from a tracked channel
- **THEN** its card shows the channel's avatar beside the title and the channel's name on its meta line

#### Scenario: Playlist video card
- **WHEN** a video in any home section comes from a tracked playlist
- **THEN** its card shows a playlist icon beside the title and the playlist's name on its meta line

#### Scenario: Continue watching shown first
- **WHEN** at least one video has been started and not finished recently
- **THEN** the home view shows a "Continue watching" section above the other sections, listing that video

#### Scenario: Continue watching card shows progress
- **WHEN** a video is shown under "Continue watching"
- **THEN** its thumbnail shows a progress bar proportional to its saved playback position over its duration

#### Scenario: Quick watches shown above latest videos
- **WHEN** at least one short unwatched video has been downloaded and is not shown in a category section
- **THEN** the home view shows a "Quick watches" section below "Continue watching" and the category sections, and above "Latest videos", listing that video

#### Scenario: Sections show a bounded number of videos
- **WHEN** more videos qualify for a section than it shows
- **THEN** "Continue watching", each category section and "Quick watches" each hold at most 6 videos, and "Latest videos" shows at most 18

#### Scenario: A video appears in one section only
- **WHEN** a video qualifies for more than one home section
- **THEN** it is shown only in the first of them, in section order

#### Scenario: Empty sections are hidden
- **WHEN** no video qualifies for "Continue watching" or "Quick watches"
- **THEN** that section, including its heading, is not shown

#### Scenario: Empty latest videos hidden
- **WHEN** every video is shown under "Continue watching", a category section or "Quick watches"
- **THEN** "Latest videos", including its heading, is not shown

#### Scenario: Nothing synced yet
- **WHEN** no section has videos
- **THEN** "Latest videos" is shown with "No videos synced yet."

#### Scenario: Home videos loading
- **WHEN** the home videos are still loading
- **THEN** the view shows placeholder sections and no section heading, "Latest videos" included

#### Scenario: Optional sections that fail to load are hidden
- **WHEN** the home videos cannot be loaded
- **THEN** "Continue watching", the category sections and "Quick watches" are not shown, and "Latest videos" shows the load error

### Requirement: Detail View Page Header
A playlist or channel detail view SHALL open with a page header showing the playlist's or channel's name, with the channel's avatar beside it for channels. Below the name it SHALL show a summary of the number of videos and, when above 0, the number of unwatched videos, also for a playlist whose count is hidden. The header SHALL offer, in this order, a sync (reconcile) action; for both, an action to mark the entry watched; for playlists an action labeled "Exclude from home" when the playlist is shown on home or "Include in home" when it is excluded, which changes that setting without confirmation, and the hide or show unwatched count action; for both, an "Edit settings" action that opens the edit channel or playlist dialog; and last, an action to delete the entry. At or above the desktop breakpoint every action but delete SHALL be a square icon-only button whose accessible name and tooltip are the action's label, followed by a vertical "⋮" control, named "More actions for <name>", whose menu offers delete, styled as destructive. Below the desktop breakpoint the header SHALL show only that "⋮" control, always visible, whose menu offers every action, sync included, with delete set apart from the others. Deleting SHALL require confirmation and, once confirmed, SHALL return the user to the home view. When syncing or changing the home setting fails, the application SHALL tell the user and leave the entry as it was.

#### Scenario: Channel page header
- **WHEN** a user opens a channel's detail view
- **THEN** the page header shows the channel's avatar, its name, a video count summary, and the sync, mark-watched, "Edit settings" and delete actions

#### Scenario: Playlist page header
- **WHEN** a user opens a playlist's detail view
- **THEN** the page header shows the playlist's name, a video count summary, and the sync, mark-watched, exclude or include in home, hide or show unwatched count, "Edit settings" and delete actions, with no avatar

#### Scenario: Header actions on a desktop viewport
- **WHEN** a user views a playlist's detail view at or above the desktop breakpoint
- **THEN** each action but delete shows as a square icon-only button, hovering one shows its label in a tooltip, and a "⋮" control follows them whose menu offers delete

#### Scenario: Page header actions on a narrow viewport
- **WHEN** a user views a playlist's detail view below the desktop breakpoint
- **THEN** the header shows only the "⋮" control beside the name, and its menu offers sync, mark-watched, the home and unwatched count actions, "Edit settings" and, set apart, delete

#### Scenario: Playlist summary includes its unwatched count
- **WHEN** a user opens the detail view of a playlist with 3 videos, 2 of them downloaded and unwatched
- **THEN** the summary shows "3 videos · 2 unwatched", whether or not the playlist's count is hidden

#### Scenario: Marking a playlist watched from its view
- **WHEN** a user chooses the mark-watched action from a playlist's page header
- **THEN** every downloaded video in the list shows a tick, and the playlist's sidebar badge disappears

#### Scenario: Editing a playlist's settings from its page header
- **WHEN** a user chooses "Edit settings" from a playlist's page header
- **THEN** the edit playlist dialog opens for that playlist

#### Scenario: Summary omits unwatched count when nothing is unwatched
- **WHEN** a detail view's channel or playlist has no unwatched videos
- **THEN** the summary shows only the video count

#### Scenario: Syncing from the page header
- **WHEN** a user activates the sync action in a detail view's page header
- **THEN** the application triggers a reconcile for that channel or playlist and shows it as in progress until it completes

#### Scenario: Deleting from the page header
- **WHEN** a user chooses the delete action from a detail view's "More actions" menu and confirms
- **THEN** the application deletes that channel or playlist and navigates to the home view

#### Scenario: Excluding a playlist from home from its page header
- **WHEN** a user chooses "Exclude from home" from a playlist's page header
- **THEN** the playlist's videos leave the home view, and the header now offers "Include in home"

#### Scenario: Including a playlist in home from its page header
- **WHEN** a user chooses "Include in home" from the page header of a playlist excluded from home
- **THEN** the playlist's videos appear in the home view's sections they qualify for, and the header now offers "Exclude from home"

### Requirement: Sidebar Row Menu
Each channel or playlist sidebar row SHALL have a menu control offering sync, mark watched, "Edit settings" (opening that entry's edit dialog) and delete. On devices with a fine pointer the control SHALL be shown while the row is hovered, while it or the row has keyboard focus, while its menu is open, and on the active row; otherwise it SHALL keep its space but not be visible. On touch devices it SHALL always be visible. Playlist rows SHALL also offer the home exclusion action ("Exclude from home"/"Include in home", without confirmation, refreshing home) and the hide/show unwatched count action. The unwatched count SHALL sit at the row's trailing edge next to the menu control. Deleting SHALL need confirmation, and mark watched SHALL ask for it as described in "Confirm Bulk Mark Watched"; no other action SHALL.

#### Scenario: Badge sits next to the menu control
- **WHEN** a row displays an unwatched count
- **THEN** the count sits at the row's trailing edge, next to the menu control's space, with no other blank space between them

#### Scenario: Menu control on hover
- **WHEN** a user with a mouse hovers a channel row that is not active
- **THEN** the row's menu control becomes visible, and it is hidden again when the pointer leaves

#### Scenario: Menu control on keyboard focus
- **WHEN** a user moves keyboard focus onto a row's menu control
- **THEN** the control is visible

#### Scenario: Marking watched takes effect without confirmation
- **WHEN** a user chooses the mark-watched action from the menu of a channel or playlist row with 1 or no unwatched videos
- **THEN** it takes effect without a confirmation prompt

#### Scenario: Marking watched asks for confirmation
- **WHEN** a user chooses the mark-watched action from the menu of a channel or playlist row with 2 or more unwatched videos
- **THEN** the application asks for confirmation before marking it watched

#### Scenario: Changing the home setting fails
- **WHEN** the request to exclude a playlist from home or include it again fails
- **THEN** the application tells the user and leaves the playlist as it was

#### Scenario: Opening a row's menu on a touch device
- **WHEN** a user on a touch device taps a sidebar row's menu control, which is always visible there
- **THEN** a menu opens listing the row's actions, without navigating to the row's detail view

#### Scenario: Syncing from the sidebar
- **WHEN** a user chooses the sync action from a sidebar row's menu
- **THEN** the application triggers a reconcile for that channel or playlist

#### Scenario: Deleting from the sidebar requires confirmation
- **WHEN** a user chooses the delete action from a sidebar row's menu
- **THEN** the application prompts for confirmation before deleting that channel or playlist

#### Scenario: Deleting the currently viewed entry
- **WHEN** a user confirms deletion of the channel or playlist whose detail view is currently open
- **THEN** the application navigates to the home view

#### Scenario: Marking a channel watched from the sidebar
- **WHEN** a user chooses the mark-watched action from a channel row's menu and confirms
- **THEN** the application marks that channel watched, and its unwatched count disappears

#### Scenario: Marking a playlist watched from the sidebar
- **WHEN** a user chooses the mark-watched action from a playlist row's menu and confirms
- **THEN** the application marks that playlist watched, its unwatched count disappears, and the counts of channels sharing its videos drop accordingly

#### Scenario: Editing a channel's settings from the sidebar
- **WHEN** a user chooses "Edit settings" from a channel row's menu
- **THEN** the edit channel dialog opens for that channel, and saving it behaves as when opened from the channel's page header

#### Scenario: Editing a playlist's settings from the sidebar
- **WHEN** a user chooses "Edit settings" from a playlist row's menu
- **THEN** the edit playlist dialog opens for that playlist, and saving it behaves as when opened from the playlist's page header

#### Scenario: Channel row menu order
- **WHEN** a user opens a channel row's menu
- **THEN** it offers, in order, sync, mark-watched, "Edit settings" and delete

#### Scenario: Playlist row menu order
- **WHEN** a user opens a playlist row's menu
- **THEN** it offers, in order, sync, mark-watched, the exclude or include in home action, the hide or show unwatched count action, "Edit settings" and delete

#### Scenario: Excluding a playlist from home from the sidebar
- **WHEN** a user chooses "Exclude from home" from the menu of a playlist shown on home
- **THEN** the playlist's videos leave the home view, and the row's menu now offers "Include in home"

#### Scenario: Including a playlist in home from the sidebar
- **WHEN** a user chooses "Include in home" from the menu of a playlist excluded from home
- **THEN** the playlist's videos appear in the home view's sections they qualify for, and the row's menu now offers "Exclude from home"

#### Scenario: Channel rows have no home exclusion action
- **WHEN** a user opens a channel row's menu
- **THEN** it offers neither "Exclude from home" nor "Include in home"

### Requirement: Sidebar Category Row Actions
Each category row SHALL provide a menu control, shown under the same conditions as the channel and playlist rows' menu control in "Sidebar Row Menu", offering "Manage sources", which navigates to that category's view with the manage sources dialog open, and "Delete", which deletes the category after the user confirms. The confirmation SHALL state how many channels and playlists the category holds and that none of them, nor any video, will be deleted. Deleting the category whose view is open SHALL return the user to the home view.

#### Scenario: Managing sources from the row menu
- **WHEN** a user chooses "Manage sources" from the "Chess" row menu
- **THEN** the "Chess" category view opens with the manage sources dialog open

#### Scenario: Deleting from the row menu
- **WHEN** a user chooses "Delete" from the menu of "Chess", which holds 4 channels and playlists
- **THEN** a confirmation states that "Chess" holds 4 channels and playlists and that none of them, nor any video, will be deleted
- **AND** confirming deletes the category and removes its row

#### Scenario: Deleting the open category
- **WHEN** a user deletes the category whose view is open
- **THEN** the application navigates to the home view

#### Scenario: Menu control on hover
- **WHEN** a user with a mouse hovers the "Chess" row
- **THEN** its menu control becomes visible

### Requirement: Category View
Each category SHALL have a view at an address of its own with a header and its videos as a grid of the same cards as the home view, each YouTube video once, newest sync first. Activating a card SHALL open the video within the category, as described in "Category Playback", instead of in its channel's or playlist's view. The header SHALL be tinted with the soft tint of the category's color and SHALL show the category's name, a summary of its number of sources, its number of videos and, when above 0, its number of unwatched videos, and the same actions layout as the channel and playlist views' page header, offering "Change color", which opens the category color picker and saves the chosen color immediately, "Manage sources", which opens the manage sources dialog, "Mark all watched" (only when the category has sources), which marks every downloaded video of the category watched after the confirmation described in "Confirm Bulk Mark Watched", and "Delete", which deletes the category after the same confirmation as the sidebar row's "Delete" and then navigates to the home view; at or above the desktop breakpoint "Delete" SHALL be in the "More actions" menu as on the channel and playlist views. When the category is known from the categories list, the header SHALL show its name and tint at once, before its videos load, with a placeholder in place of the summary; the videos SHALL show as placeholders as described in "Loading Placeholders" until they load. A category with no sources SHALL say so and offer a control that opens the manage sources dialog. A category with sources but without downloaded videos SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

#### Scenario: Category videos
- **WHEN** a user opens the "Chess" view, which holds 2 sources with 14 downloaded videos, 3 of them unwatched
- **THEN** the view shows "Chess", the summary "2 sources · 14 videos · 3 unwatched", and the videos as cards, newest sync first

#### Scenario: Card opens the video in the category
- **WHEN** a user activates a video card on the "Chess" view
- **THEN** the application stays on the "Chess" view and plays that video, as described in "Category Playback"

#### Scenario: Header tinted
- **WHEN** a user opens the "Chess" view and "Chess" is colored "#0193bf"
- **THEN** the header's background is the soft tint of that color's hue

#### Scenario: Header while loading
- **WHEN** a user opens the "Chess" view from the sidebar and its videos are still loading
- **THEN** the header already shows "Chess" with its tint and actions, a placeholder in place of the summary, and placeholder cards below

#### Scenario: Changing the color
- **WHEN** a user activates "Change color" on the "Chess" view and chooses "Violet"
- **THEN** "Chess" is saved with the color "#9870c5", and the header, the sidebar dot and the home chips of its videos show it

#### Scenario: Mark all watched
- **WHEN** a user chooses "Mark all watched" from a category view's header and confirms
- **THEN** every card shows as watched, and the category's sidebar count disappears promptly

#### Scenario: Deleting from the category view
- **WHEN** a user chooses "Delete" from the "Chess" view's header and confirms
- **THEN** the category is deleted and the application navigates to the home view

#### Scenario: Empty category
- **WHEN** a user opens a category with no sources
- **THEN** the view says it holds no channels or playlists yet and offers a control that opens the manage sources dialog

#### Scenario: Unknown category
- **WHEN** a user opens the view of a category that does not exist
- **THEN** the view says the category was not found

### Requirement: Sidebar Categories List
The sidebar SHALL show a "Categories" section above the Channels section listing every category, never trimmed, each row showing a dot in the category's color, the category's name and, when its unwatched count is above 0, that count. Selecting a row SHALL navigate to that category's view, and while the user is on it, its row SHALL be marked active.

#### Scenario: Categories listed above channels
- **WHEN** the categories "Chess" and "Sports" exist
- **THEN** the sidebar shows a "Categories" section above "Channels" listing both

#### Scenario: Color dot
- **WHEN** the category "Chess" is colored "#0193bf"
- **THEN** its row shows a "#0193bf" dot before its name

#### Scenario: Every category shown
- **WHEN** 25 categories exist and the Categories section is unfolded
- **THEN** all 25 are listed, with no show-more control

#### Scenario: Category badge
- **WHEN** the category "Chess" has 12 unwatched videos and "Science" has none
- **THEN** the "Chess" row shows the count 12 and the "Science" row shows no count

#### Scenario: Opening a category
- **WHEN** a user selects the "Chess" row
- **THEN** the application navigates to the "Chess" category view and marks the row active

#### Scenario: Search filters categories
- **WHEN** the categories "Chess" and "Science" exist and a user types "sci" in the header search
- **THEN** the search's Categories group lists only "Science", and the sidebar's Categories section still lists both

### Requirement: Category Color Picker
The category color picker SHALL offer the 10 palette colors as swatches in palette order, each a radio option named Mustard, Orange, Coral, Rose, Violet, Periwinkle, Sky, Teal, Olive and Taupe, plus a "Custom" option with a hue slider. Every custom color SHALL have the palette's solid lightness and saturation, so only the hue changes. The chosen option SHALL be shown checked. A color not in the palette SHALL show as "Custom" with the slider at its hue.

#### Scenario: Choosing a swatch
- **WHEN** a user chooses the "Sky" swatch
- **THEN** "Sky" is shown checked and the chosen color is "#0193bf"

#### Scenario: Clicking a swatch
- **WHEN** a user clicks anywhere on the "Violet" swatch
- **THEN** "Violet" is shown checked

#### Scenario: Choosing a custom hue
- **WHEN** a user chooses "Custom" and moves the hue slider
- **THEN** the chosen color follows the hue and keeps the palette's solid lightness and saturation

#### Scenario: Existing custom color
- **WHEN** the picker opens for a category whose color is not in the palette
- **THEN** "Custom" is shown checked with the slider at that color's hue

### Requirement: Tasks View Tab Bar Affordance
The tasks view's tab bar SHALL make each tab look clickable and SHALL make the selected tab unmistakable. The tasks view SHALL be limited to a readable width and aligned to the left edge of the content area, like the other views, and the tab bar SHALL span exactly the width of the task list, its tabs sharing that width equally. The tab bar SHALL be drawn as a row of tabs over a horizontal rule, with the selected tab marked by an underline in the application's primary colour and its label in the full foreground colour and a heavier weight; unselected tabs SHALL show a muted label. Hovering an unselected tab SHALL visibly change its background and label colour. Each tab SHALL show an icon for its family, its name, and its count as a pill, with the selected tab's pill in the primary colour. On narrow screens each tab SHALL show only its icon and count, keeping its name as its accessible name, so that all tabs fit without horizontal scrolling. While at least one task is running, the **Active** tab SHALL show a live indicator beside its count. Each tab SHALL be at least 40 pixels tall and SHALL show a visible focus ring when focused from the keyboard. The tab bar SHALL look the same whatever the operating system's colour-scheme preference.

#### Scenario: Selected tab is marked
- **WHEN** a user views the tasks view with the Syncs tab selected
- **THEN** the Syncs tab shows the primary-coloured underline, a full-colour heavier label, and a primary-coloured count pill, and no other tab does

#### Scenario: Hovering an unselected tab
- **WHEN** a user hovers an unselected tab with a pointer
- **THEN** that tab's background and label colour change

#### Scenario: Live indicator on Active
- **WHEN** at least one task is running
- **THEN** the Active tab shows a live indicator beside its count

#### Scenario: No live indicator when idle
- **WHEN** no task is running
- **THEN** the Active tab shows no live indicator

#### Scenario: Keyboard focus
- **WHEN** a user moves keyboard focus onto a tab
- **THEN** that tab shows a visible focus ring

#### Scenario: Wide screen
- **WHEN** the tasks view is shown on a wide desktop screen
- **THEN** the view is left-aligned at a readable width, its left edge matching the other views' content, and the tab bar's rule and the task list share the same left and right edges

#### Scenario: Narrow screen
- **WHEN** the tasks view is shown on a phone-width screen
- **THEN** every tab shows its icon and count without its visible name, all four tabs are visible without scrolling, and each tab is still announced by its name to assistive technology

## REMOVED Requirements

### Requirement: Home Card Category Tint
**Reason**: Pastel card tints fought the thumbnails, carried no information in category sections, and turned into bands that read as a bug for videos in several categories.
**Migration**: Categories show as text chips on mixed home sections ("Home Card Category Chips"); cards are frameless ("Video Card Presentation").

### Requirement: Sidebar Search Field
**Reason**: Replaced by the header's global search, which finds categories, channels, playlists and videos.
**Migration**: Use the header "Search" control or ⌘K/Ctrl+K ("Global Search").

### Requirement: Sidebar Search Filtering
**Reason**: Replaced by the header's global search.
**Migration**: Use "Global Search"; its Categories, Channels and Playlists groups match names the same way, ignoring case.
