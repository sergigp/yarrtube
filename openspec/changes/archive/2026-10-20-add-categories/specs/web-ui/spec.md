## ADDED Requirements

### Requirement: Sidebar Categories Section
The sidebar SHALL show a "Categories" section above the Channels section. Its heading SHALL fold and unfold the section like the other sidebar sections and, while folded, SHALL show the unwatched total across all categories as a badge styled like the row badges, at the heading's trailing edge, when it is above 0. While unfolded, the section SHALL offer an "Add category" control placed like the other sections' add controls: a full-width labelled row beneath the heading while no category exists, otherwise (categories exist, or they are still loading) a compact "+" button at the trailing edge of the heading, outside its fold toggle, with the accessible name and tooltip "Add category". The section SHALL list every category, never trimmed, each row showing the category's name and, when its unwatched count is above 0, a badge with that count. Selecting a category row SHALL navigate to that category's view. When the user is on a category's view, its row SHALL be marked active; when the section is folded, its heading SHALL be marked active instead. The sidebar search SHALL NOT filter categories. With no categories, the unfolded section SHALL show only its heading and the full-width "Add category" row.

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

#### Scenario: Folded categories total
- **WHEN** the Categories section is folded and the unwatched total across categories is 16
- **THEN** the heading shows an unwatched badge of 16

#### Scenario: Search leaves categories alone
- **WHEN** a user types in the sidebar search field
- **THEN** the Categories section is unchanged

#### Scenario: No categories yet
- **WHEN** no category exists and the Categories section is unfolded
- **THEN** the section shows its heading and a full-width "Add category" row, and no category rows

#### Scenario: Adding from the heading once categories exist
- **WHEN** at least one category exists and the Categories section is unfolded
- **THEN** the heading shows a "+" control named "Add category", and no "Add category" row is shown

### Requirement: Sidebar Category Ordering
The sidebar's Categories section SHALL list categories with an unwatched count above 0 first, ordered by unwatched count from highest to lowest, with equal counts ordered by name, ignoring case. The categories with no unwatched videos SHALL follow, ordered by name, ignoring case.

#### Scenario: Unread categories first, most unread first
- **WHEN** the categories are "Alpha" (0 unwatched), "Bravo" (4 unwatched), "Charlie" (12 unwatched) and "Delta" (0 unwatched)
- **THEN** the Categories section lists them in the order "Charlie", "Bravo", "Alpha", "Delta"

#### Scenario: Equal counts by name
- **WHEN** "sports" and "Cooking" both have 4 unwatched videos
- **THEN** "Cooking" is listed before "sports"

### Requirement: Category Badges Stay Current
The sidebar's category badges, their order and the folded Categories total SHALL update without a manual page reload, and promptly after any action in the application that changes a channel's or playlist's unwatched videos (the same actions that update channel badges promptly), after marking a category watched, and after changing a category's members.

#### Scenario: Watching a video of a category
- **WHEN** a user finishes watching an unwatched video of a channel in "Chess"
- **THEN** the "Chess" badge decreases by one promptly without a page reload

#### Scenario: Adding a member
- **WHEN** a user adds a channel with 3 unwatched videos to the empty category "Cooking"
- **THEN** the "Cooking" row promptly shows a badge with 3

### Requirement: Sidebar Category Row Actions
Each category row SHALL provide an always-visible menu control offering "Manage sources", which navigates to that category's view with the manage sources dialog open, and "Delete", which deletes the category after the user confirms. The confirmation SHALL state how many channels and playlists the category holds and that none of them, nor any video, will be deleted. Deleting the category whose view is open SHALL return the user to the home view.

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

### Requirement: Add Category Dialog
The "Add category" control SHALL open a dialog asking only for a name. Creating SHALL be possible once the name is not blank. On success, the dialog SHALL close and the application SHALL navigate to the new category's view with the manage sources dialog open. When creation fails (including a name already taken), the dialog SHALL stay open and show the error. On viewports narrower than the desktop breakpoint, activating "Add category" SHALL close the sidebar before the dialog is shown.

#### Scenario: Creating a category
- **WHEN** a user activates "Add category", enters "Chess" and creates it
- **THEN** the dialog closes and the "Chess" category view opens with the manage sources dialog open and its search field focused

#### Scenario: Blank name
- **WHEN** the name field is empty or blank
- **THEN** the create action is disabled

#### Scenario: Name already taken
- **WHEN** a user creates a category with a name another category already has
- **THEN** the dialog stays open and says the name is already taken

### Requirement: Category View
Each category SHALL have a view at an address of its own with a header and its videos as a grid of the same cards as the home view, each YouTube video once, newest sync first. The header SHALL show the category's name, a summary of its number of sources, its number of videos and, when above 0, its number of unwatched videos, and a "More actions for <name>" menu control like the channel and playlist views' offering "Mark all watched" (only when the category has sources), which marks every downloaded video of the category watched without confirmation, "Manage sources", which opens the manage sources dialog, and "Delete", which deletes the category after the same confirmation as the sidebar row's "Delete" and then navigates to the home view. A category with no sources SHALL say so and offer a control that opens the manage sources dialog. A category with sources but without downloaded videos SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

#### Scenario: Category videos
- **WHEN** a user opens the "Chess" view, which holds 2 sources with 14 downloaded videos, 3 of them unwatched
- **THEN** the view shows "Chess", the summary "2 sources · 14 videos · 3 unwatched", and the videos as cards, newest sync first

#### Scenario: Mark all watched
- **WHEN** a user chooses "Mark all watched" from a category view's actions menu
- **THEN** every card shows as watched, and the category's sidebar badge disappears promptly

#### Scenario: Deleting from the category view
- **WHEN** a user chooses "Delete" from the "Chess" view's actions menu and confirms
- **THEN** the category is deleted and the application navigates to the home view

#### Scenario: Empty category
- **WHEN** a user opens a category with no sources
- **THEN** the view says it holds no channels or playlists yet and offers a control that opens the manage sources dialog

#### Scenario: Unknown category
- **WHEN** a user opens the view of a category that does not exist
- **THEN** the view says the category was not found

### Requirement: Rename Category In Place
The category view's name SHALL be shown as a heading, not as a field, next to an icon control with the accessible name and tooltip "Rename". Activating it SHALL replace the heading with a field holding the name, focused and selected. Pressing Enter or moving focus away SHALL save the name; pressing Escape SHALL cancel and show the saved name. A blank name or a name another category already has SHALL be rejected with a message, keeping the field open with the typed name until the user saves a valid name or cancels. An unchanged name SHALL close the field without saving.

#### Scenario: Renaming
- **WHEN** a user activates "Rename" on the "Chess" view, types "Board games" and presses Enter
- **THEN** the heading shows "Board games" and so does the sidebar row

#### Scenario: Cancelling a rename
- **WHEN** a user activates "Rename", changes the name and presses Escape
- **THEN** the heading shows the saved name and nothing is saved

#### Scenario: Rename rejected
- **WHEN** a user renames "Chess" to the name of another category
- **THEN** the view says the name is already taken and the category keeps its saved name

### Requirement: Manage Sources Dialog
The manage sources dialog SHALL be titled "Manage sources in <name>" and SHALL show every tracked channel and playlist as a chip, channels under a "Channels" group and playlists under a "Playlists" group, each group ordered by name ignoring case. Channel chips SHALL show the channel's avatar; playlist chips a playlist icon. Each chip SHALL be a toggle whose pressed state means the source is in the category, shown with a check mark and a tinted fill, while a source not in it is shown outlined with a plus mark. Toggling a chip SHALL NOT change the order of the chips. A search field, focused when the dialog opens, SHALL filter the chips by name ignoring case, and pressing Enter in it while exactly one chip matches SHALL toggle that chip. Filter controls "All", "In <name>" and "Not in <name>", the last two showing how many sources they hold, SHALL narrow the chips by state. A group with no chip left to show SHALL be hidden. Toggling SHALL only stage changes: the dialog SHALL summarise them (such as "+2 added, −1 removed"), its "Save" control SHALL be disabled until something is staged, and saving SHALL make the category's sources exactly the pressed chips in one change, then close the dialog. When saving fails, the dialog SHALL stay open, keep the staged changes and say why. Cancelling or dismissing the dialog with staged changes SHALL ask to discard them first. With no channel or playlist tracked, the dialog SHALL say so. Chips SHALL be reachable with the arrow keys within a group. On viewports narrower than the desktop breakpoint the dialog SHALL fill the screen; the search, the filter controls and the save controls SHALL stay visible while the chips scroll.

#### Scenario: Chips show membership
- **WHEN** a user opens the dialog of "Chess", which holds the channel "Hikaru", and "GothamChess" and the playlist "Endgame course" are also tracked
- **THEN** "Channels" shows "GothamChess" unpressed and "Hikaru" pressed, and "Playlists" shows "Endgame course" unpressed

#### Scenario: Staging and saving
- **WHEN** a user presses "GothamChess" and "Endgame course", unpresses "Hikaru" and saves
- **THEN** the summary read "+2 added, −1 removed" before saving, and afterwards "Chess" holds exactly "GothamChess" and "Endgame course"

#### Scenario: Order stays put
- **WHEN** a user toggles a chip
- **THEN** every chip keeps its place

#### Scenario: Searching
- **WHEN** a user types "ver" in the search field
- **THEN** only chips whose name contains "ver", ignoring case, are shown

#### Scenario: Enter toggles the only match
- **WHEN** the search leaves exactly one chip and the user presses Enter
- **THEN** that chip is toggled and the search field keeps focus

#### Scenario: Filtering by state
- **WHEN** a user chooses "In Chess"
- **THEN** only pressed chips are shown, staged ones included

#### Scenario: Nothing to save
- **WHEN** no chip has been toggled, or every toggle has been undone
- **THEN** "Save" is disabled

#### Scenario: Discarding staged changes
- **WHEN** a user with staged changes cancels the dialog
- **THEN** the user is asked to discard them, and confirming closes the dialog without saving

#### Scenario: Save fails
- **WHEN** saving fails
- **THEN** the dialog stays open with the staged changes and says why

#### Scenario: Nothing tracked
- **WHEN** a user opens the dialog with no channel or playlist tracked
- **THEN** the dialog says there are no channels or playlists to add yet

### Requirement: Categories Field In Add And Edit Dialogs
The add channel, add playlist, edit channel and edit playlist dialogs SHALL offer a "Categories" field showing the source's categories as removable chips and letting the user pick further existing categories by name, filtered as they type, ignoring case. When the typed text matches no category name, the field SHALL offer to create a category with that name; choosing it SHALL create the category at once and add it to the field. Saving an add dialog SHALL track the source and then make it a member of exactly the chosen categories. Saving an edit dialog SHALL make the source a member of exactly the chosen categories, and a change of categories alone SHALL count as a change to save. When the source is saved but its categories cannot be, the dialog SHALL say so and keep the source saved.

#### Scenario: Choosing categories when adding a channel
- **WHEN** a user adds a channel with "Chess" and "Streams" chosen in the Categories field
- **THEN** the channel is tracked and is a member of "Chess" and "Streams"

#### Scenario: Creating a category from the field
- **WHEN** a user types "Speedcubing", which matches no category, and chooses to create it
- **THEN** the category "Speedcubing" exists and is shown as a chip in the field

#### Scenario: Editing a source's categories
- **WHEN** a user removes the "Chess" chip from a channel's edit dialog and saves
- **THEN** the channel is no longer a member of "Chess" and stays in its other categories

#### Scenario: Categories are the only change
- **WHEN** a user changes only the Categories field in an edit dialog
- **THEN** the dialog can be saved and saving updates the categories

#### Scenario: Categories fail to save
- **WHEN** a user adds a channel with categories chosen and the channel is tracked but its categories cannot be saved
- **THEN** the dialog says the channel was added but its categories were not saved

## MODIFIED Requirements

### Requirement: Sidebar Add Entries
The sidebar's Channels section SHALL offer an "Add channel" control and its Playlists section an "Add playlist" control, visible whenever the section is unfolded. When the section has loaded with nothing tracked, the control SHALL be a full-width labelled row placed beneath the section heading. Otherwise (the section has rows, is still loading, or failed to load) the control SHALL be a compact "+" button at the trailing edge of the section heading, outside the heading's fold toggle, with the accessible name "Add channel" or "Add playlist" and that name as its tooltip. A folded section SHALL NOT show its add control. "Add channel" SHALL open the add channel dialog and "Add playlist" SHALL open the add playlist dialog. On viewports narrower than the desktop breakpoint, activating either control SHALL close the sidebar before the dialog is shown.

#### Scenario: Opening the add channel dialog from the sidebar
- **WHEN** a user activates "Add channel" in the sidebar's Channels section
- **THEN** the add channel dialog opens

#### Scenario: Opening the add playlist dialog from the sidebar
- **WHEN** a user activates "Add playlist" in the sidebar's Playlists section
- **THEN** the add playlist dialog opens

#### Scenario: Add entries with nothing tracked
- **WHEN** no channels or playlists are tracked and both sections are unfolded
- **THEN** both sections show a full-width labelled add row beneath their heading

#### Scenario: Heading add control once tracked
- **WHEN** at least one channel is tracked and the Channels section is unfolded
- **THEN** the "Channels" heading shows a "+" control named "Add channel", and no "Add channel" row is shown

#### Scenario: Folded section hides its add entry
- **WHEN** the Channels section is folded
- **THEN** no "Add channel" control is shown

#### Scenario: Adding from the mobile sidebar
- **WHEN** a user on a mobile-width viewport opens the sidebar and activates "Add channel" or "Add playlist"
- **THEN** the sidebar closes and the matching add dialog is shown

### Requirement: Folded Section Unwatched Total
A folded Channels section SHALL show, at its heading's trailing edge and as a badge styled like the rows' unwatched badges, the sum of its channels' unwatched counts, when that sum is above 0. No total SHALL be shown while the section is unfolded, while it is folded with a sum of 0, or for the Playlists section. The total SHALL update together with the channel badges.

#### Scenario: Folded channels with unwatched videos
- **WHEN** the Channels section is folded and its channels have 3, 0 and 20 unwatched videos
- **THEN** the "Channels" heading shows an unwatched badge of 23

#### Scenario: Folded channels with nothing unwatched
- **WHEN** the Channels section is folded and no channel has unwatched videos
- **THEN** the "Channels" heading shows no total

#### Scenario: Unfolded section shows no total
- **WHEN** the Channels section is unfolded
- **THEN** the "Channels" heading shows no total

#### Scenario: Playlists show no total
- **WHEN** the Playlists section is folded
- **THEN** the "Playlists" heading shows no total

#### Scenario: Total follows watching
- **WHEN** the Channels section is folded and a user finishes watching one of a channel's unwatched videos
- **THEN** the total decreases by one promptly without a page reload
