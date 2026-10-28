# Spec Delta

## REMOVED Requirements

### Requirement: Sidebar Search
**Reason**: Replaced by "Sidebar Search Field" and "Sidebar Search Filtering": the search now also filters categories and sits above them, which contradicts this requirement's scenarios.
**Migration**: None; see those requirements.

### Requirement: Sidebar Categories Section
**Reason**: Replaced by "Sidebar Categories List", "Sidebar Categories Heading" and "Sidebar Add Category Control": the sidebar search now filters categories, which contradicts this requirement's "Search leaves categories alone" scenario.
**Migration**: None; see those requirements.

## MODIFIED Requirements

### Requirement: Detail View Page Header
A playlist or channel detail view SHALL open with a page header showing the playlist's or channel's name, with the channel's avatar beside it for channels. Below the name it SHALL show a summary of the number of videos and, when above 0, the number of unwatched videos, also for a playlist whose count is hidden. The header SHALL offer, in this order, a sync (reconcile) action; for both, an action to mark the entry watched; for playlists an action labeled "Exclude from home" when the playlist is shown on home or "Include in home" when it is excluded, which changes that setting without confirmation, and the hide or show unwatched count action; for both, an "Edit settings" action that opens the edit channel or playlist dialog; and last, an action to delete the entry. At or above the desktop breakpoint each action SHALL be a square icon-only button whose accessible name and tooltip are the action's label, with delete styled as destructive, and there SHALL be no "⋮" control. Below the desktop breakpoint the header SHALL show only an always-visible vertical "⋮" control, named "More actions for <name>", whose menu offers every action, sync included, with delete set apart from the others. Deleting SHALL require confirmation and, once confirmed, SHALL return the user to the home view. When syncing or changing the home setting fails, the application SHALL tell the user and leave the entry as it was.

#### Scenario: Channel page header
- **WHEN** a user opens a channel's detail view
- **THEN** the page header shows the channel's avatar, its name, a video count summary, and the sync, mark-watched, "Edit settings" and delete actions

#### Scenario: Playlist page header
- **WHEN** a user opens a playlist's detail view
- **THEN** the page header shows the playlist's name, a video count summary, and the sync, mark-watched, exclude or include in home, hide or show unwatched count, "Edit settings" and delete actions, with no avatar

#### Scenario: Header actions on a desktop viewport
- **WHEN** a user views a playlist's detail view at or above the desktop breakpoint
- **THEN** each action shows as a square icon-only button, hovering one shows its label in a tooltip, and no "⋮" control is shown

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
- **WHEN** a user chooses the delete action from a detail view's page header and confirms
- **THEN** the application deletes that channel or playlist and navigates to the home view

#### Scenario: Excluding a playlist from home from its page header
- **WHEN** a user chooses "Exclude from home" from a playlist's page header
- **THEN** the playlist's videos leave the home view, and the header now offers "Include in home"

#### Scenario: Including a playlist in home from its page header
- **WHEN** a user chooses "Include in home" from the page header of a playlist excluded from home
- **THEN** the playlist's videos appear in the home view's sections they qualify for, and the header now offers "Exclude from home"

### Requirement: Category View
Each category SHALL have a view at an address of its own with a header and its videos as a grid of the same cards as the home view, each YouTube video once, newest sync first. The header SHALL show the category's name, a summary of its number of sources, its number of videos and, when above 0, its number of unwatched videos, and the same actions layout as the channel and playlist views' page header, offering "Manage sources", which opens the manage sources dialog, "Mark all watched" (only when the category has sources), which marks every downloaded video of the category watched without confirmation, and "Delete", which deletes the category after the same confirmation as the sidebar row's "Delete" and then navigates to the home view. A category with no sources SHALL say so and offer a control that opens the manage sources dialog. A category with sources but without downloaded videos SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

#### Scenario: Category videos
- **WHEN** a user opens the "Chess" view, which holds 2 sources with 14 downloaded videos, 3 of them unwatched
- **THEN** the view shows "Chess", the summary "2 sources · 14 videos · 3 unwatched", and the videos as cards, newest sync first

#### Scenario: Mark all watched
- **WHEN** a user chooses "Mark all watched" from a category view's header
- **THEN** every card shows as watched, and the category's sidebar badge disappears promptly

#### Scenario: Deleting from the category view
- **WHEN** a user chooses "Delete" from the "Chess" view's header and confirms
- **THEN** the category is deleted and the application navigates to the home view

#### Scenario: Empty category
- **WHEN** a user opens a category with no sources
- **THEN** the view says it holds no channels or playlists yet and offers a control that opens the manage sources dialog

#### Scenario: Unknown category
- **WHEN** a user opens the view of a category that does not exist
- **THEN** the view says the category was not found

### Requirement: Mark Channel Watched From Channel View
A channel detail view SHALL provide an action to mark the channel watched, which takes effect without confirmation. The action SHALL sit among the page header's actions.

#### Scenario: Marking a channel watched from its view
- **WHEN** a user chooses the mark-watched action from a channel's page header
- **THEN** every downloaded video in the list shows a tick, and the channel's sidebar badge disappears

### Requirement: Hide Playlist Unwatched Count
Each playlist's sidebar row menu and page header SHALL offer an action labeled "Hide unwatched count" when the playlist's count is shown, or "Show unwatched count" when it is hidden. The action SHALL change that setting without confirmation. When changing it fails, the application SHALL tell the user and leave the playlist as it was. Channel menus and page headers SHALL offer neither action.

#### Scenario: Hiding a playlist's count from the sidebar
- **WHEN** a user chooses "Hide unwatched count" from a playlist row's menu
- **THEN** the row's badge disappears, the playlist sorts among the caught-up playlists, and the menu now offers "Show unwatched count"

#### Scenario: Showing a playlist's count from its page header
- **WHEN** a user chooses "Show unwatched count" from the page header of a playlist whose count is hidden
- **THEN** the playlist's sidebar badge shows its unwatched count, and the header now offers "Hide unwatched count"

#### Scenario: Changing the setting fails
- **WHEN** the request to change a playlist's hide setting fails
- **THEN** the application tells the user, and the playlist's badge and header actions stay as they were

#### Scenario: Channel menus have no hide count action
- **WHEN** a user opens a channel row's menu or a channel's page header
- **THEN** it offers neither "Hide unwatched count" nor "Show unwatched count"

## ADDED Requirements

### Requirement: Sidebar Search Field
When the number of categories plus tracked channels plus tracked playlists is above 15, the sidebar SHALL show a single search field, named "Search categories, channels and playlists", above all its sections, the Categories section included. At 15 or fewer the field SHALL NOT be shown.

#### Scenario: Search field hidden for a short list
- **WHEN** 10 channels and 5 playlists are tracked and no category exists
- **THEN** the sidebar shows no search field

#### Scenario: Search field shown for a long list
- **WHEN** 12 channels and 4 playlists are tracked
- **THEN** the sidebar shows a search field above the Categories section

#### Scenario: Categories count toward the threshold
- **WHEN** 10 channels are tracked and 6 categories exist
- **THEN** the sidebar shows a search field

### Requirement: Sidebar Search Filtering
While the sidebar search field contains text, the Categories, Channels and Playlists sections SHALL each show every entry whose name contains that text, ignoring case, in the section's usual order and regardless of whether the section is folded. A section with no matching entry SHALL be hidden; when no section has one, the sidebar SHALL say that nothing matches. Clearing the field SHALL restore each section's folded or unfolded view. Searching SHALL NOT change any remembered fold state.

#### Scenario: Matching across sections
- **WHEN** a user types "ver" and the channel "Veritasium", the playlist "Universe Overview" and the category "Universe" exist
- **THEN** the Channels section shows "Veritasium", the Playlists section shows "Universe Overview" and the Categories section shows "Universe"

#### Scenario: Matches in a folded section
- **WHEN** the Channels section is folded and a user types text matching the channel "Veritasium"
- **THEN** the Channels section shows "Veritasium"

#### Scenario: Section without matches is hidden
- **WHEN** a user's search text matches only channels
- **THEN** neither the Categories nor the Playlists section is shown

#### Scenario: Nothing matches
- **WHEN** a user types "zzz" and no category, channel or playlist name contains it
- **THEN** the sidebar says nothing matches "zzz"

#### Scenario: Clearing the search
- **WHEN** a user clears the search field
- **THEN** every section returns to the folded or unfolded view it had before searching

### Requirement: Sidebar Categories List
The sidebar SHALL show a "Categories" section above the Channels section listing every category, never trimmed, each row showing the category's name and, when its unwatched count is above 0, a badge with that count. Selecting a row SHALL navigate to that category's view, and while the user is on it, its row SHALL be marked active. The sidebar search SHALL filter the section like the other sections.

#### Scenario: Categories listed above channels
- **WHEN** the categories "Chess" and "Sports" exist
- **THEN** the sidebar shows a "Categories" section above "Channels" listing both

#### Scenario: Every category shown
- **WHEN** 25 categories exist and the Categories section is unfolded
- **THEN** all 25 are listed, with no show-more control

#### Scenario: Category badge
- **WHEN** the category "Chess" has 12 unwatched videos and "Science" has none
- **THEN** the "Chess" row shows a badge with 12 and the "Science" row shows no badge

#### Scenario: Opening a category
- **WHEN** a user selects the "Chess" row
- **THEN** the application navigates to the "Chess" category view and marks the row active

#### Scenario: Search filters categories
- **WHEN** the categories "Chess" and "Science" exist and a user types "sci" in the sidebar search field
- **THEN** the Categories section lists only "Science"

### Requirement: Sidebar Categories Heading
The Categories section's heading SHALL fold and unfold the section like the other sidebar sections. While folded, it SHALL show the unwatched total across all categories as a badge styled like the row badges, at its trailing edge, when that total is above 0, and SHALL be marked active while the user is on a category's view.

#### Scenario: Folded categories total
- **WHEN** the Categories section is folded and the unwatched total across categories is 16
- **THEN** the heading shows an unwatched badge of 16

#### Scenario: Folded heading marked active
- **WHEN** the Categories section is folded and the user is on the "Chess" category view
- **THEN** the Categories heading is marked active

### Requirement: Sidebar Add Category Control
While unfolded, the Categories section SHALL offer an "Add category" control: a full-width labelled row beneath the heading while no category exists, and no category rows; otherwise (categories exist, or are still loading) a compact "+" button at the heading's trailing edge, outside its fold toggle, with the accessible name and tooltip "Add category".

#### Scenario: No categories yet
- **WHEN** no category exists and the Categories section is unfolded
- **THEN** the section shows its heading and a full-width "Add category" row, and no category rows

#### Scenario: Adding from the heading once categories exist
- **WHEN** at least one category exists and the Categories section is unfolded
- **THEN** the heading shows a "+" control named "Add category", and no "Add category" row is shown

### Requirement: Player Keyboard Seeking
While a downloaded video is in a playlist or channel detail view's player, pressing ← or → without modifier keys SHALL seek it 10 seconds back or forward, kept within the video, instead of the browser's own step. Presses typed into a text field, or already handled by an open menu or dialog, SHALL NOT seek.

#### Scenario: Seeking forward
- **WHEN** a video is 30 seconds in and the user presses →
- **THEN** it is 40 seconds in

#### Scenario: Seeking back near the start
- **WHEN** a video is 4 seconds in and the user presses ←
- **THEN** it is at the start

#### Scenario: Typing in a field
- **WHEN** the focus is in the sidebar search field and the user presses ←
- **THEN** the video does not seek

### Requirement: Player Seek Feedback
Each keyboard seek SHALL show feedback over the side of the player sought towards: chevrons pointing that way and the seconds sought, e.g. "10 seconds". Presses towards the same direction while it shows SHALL add up ("20 seconds" after two); a press the other way SHALL start again from 10. The feedback SHALL fade out on its own within a second of the last press.

#### Scenario: Feedback for one press
- **WHEN** the user presses → in the player
- **THEN** the right side of the player shows forward chevrons and "10 seconds", which fade away shortly after

#### Scenario: Presses add up
- **WHEN** the user presses → twice in quick succession
- **THEN** the feedback shows "20 seconds"

#### Scenario: Changing direction
- **WHEN** the feedback shows "20 seconds" forward and the user presses ←
- **THEN** the left side of the player shows "10 seconds"
