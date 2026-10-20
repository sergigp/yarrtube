## ADDED Requirements

### Requirement: Sidebar Categories Section
The sidebar SHALL show a "Categories" section above the Channels section. Its heading SHALL fold and unfold the section like the other sidebar sections, SHALL carry a manage control with the accessible name "Manage categories" that navigates to the categories management view without folding or unfolding the section, and, while folded, SHALL show in a muted style the unwatched total across all categories when it is above 0. While unfolded, the section SHALL show an "Add category" control between its heading and its list, and SHALL list every category, never trimmed, each row showing the category's name and, when its unwatched count is above 0, a badge with that count. Selecting a category row SHALL navigate to that category's view. When the user is on a category's view, its row SHALL be marked active; when the section is folded, its heading SHALL be marked active instead. The sidebar search SHALL NOT filter categories. With no categories, the unfolded section SHALL show only its heading and the "Add category" control.

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

#### Scenario: Manage control
- **WHEN** a user activates "Manage categories" in the Categories heading
- **THEN** the application navigates to the categories management view and the section's fold state is unchanged

#### Scenario: Folded categories total
- **WHEN** the Categories section is folded and the unwatched total across categories is 16
- **THEN** the heading shows a muted total of 16

#### Scenario: Search leaves categories alone
- **WHEN** a user types in the sidebar search field
- **THEN** the Categories section is unchanged

#### Scenario: No categories yet
- **WHEN** no category exists and the Categories section is unfolded
- **THEN** the section shows its heading and the "Add category" control, and no rows

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
Each category row SHALL provide an always-visible menu control offering "Manage", which navigates to the categories management view with that category selected, and "Delete", which deletes the category after the user confirms. The confirmation SHALL state how many channels and playlists the category holds and that none of them, nor any video, will be deleted. Deleting the category whose view is open SHALL return the user to the home view.

#### Scenario: Managing from the row menu
- **WHEN** a user chooses "Manage" from the "Chess" row menu
- **THEN** the categories management view opens with "Chess" selected

#### Scenario: Deleting from the row menu
- **WHEN** a user chooses "Delete" from the menu of "Chess", which holds 4 channels and playlists
- **THEN** a confirmation states that "Chess" holds 4 channels and playlists and that none of them, nor any video, will be deleted
- **AND** confirming deletes the category and removes its row

#### Scenario: Deleting the open category
- **WHEN** a user deletes the category whose view is open
- **THEN** the application navigates to the home view

### Requirement: Add Category Dialog
The "Add category" control SHALL open a dialog asking only for a name. Creating SHALL be possible once the name is not blank. On success, the dialog SHALL close and the application SHALL navigate to the categories management view with the new category selected and focus on its add-members search field. When creation fails (including a name already taken), the dialog SHALL stay open and show the error. On viewports narrower than the desktop breakpoint, activating "Add category" SHALL close the sidebar before the dialog is shown.

#### Scenario: Creating a category
- **WHEN** a user activates "Add category", enters "Chess" and creates it
- **THEN** the dialog closes and the management view opens with "Chess" selected and its add-members search field focused

#### Scenario: Blank name
- **WHEN** the name field is empty or blank
- **THEN** the create action is disabled

#### Scenario: Name already taken
- **WHEN** a user creates a category with a name another category already has
- **THEN** the dialog stays open and says the name is already taken

### Requirement: Categories Management View
The application SHALL provide a categories management view, reachable from the sidebar's manage control, from a category row's "Manage" action and from the header Settings menu. It SHALL list every category by name, ignoring case, each with its number of members, and SHALL offer a "New category" control that opens the add category dialog. Selecting a category SHALL show its details:
- its name, editable in place and saved when the field loses focus or Enter is pressed; a blank name or a name already taken SHALL be rejected with a message, keeping the saved name;
- an "In <name>" list grouping its members by kind ("Channels", then "Playlists"), each member by name ordered ignoring case, channels with their avatar, each with a "Remove" control that removes it at once;
- an add-members picker listing the tracked channels and playlists that are not members, filtered by a search field by name ignoring case, with a checkbox per source and an "Add selected" control that adds every checked source at once;
- a "Delete" control that deletes the category after the same confirmation as the sidebar row's "Delete", then shows the view with no category selected;
- a link to the category's view.
On viewports at or above the desktop breakpoint the list and the selected category's details SHALL be shown side by side. On narrower viewports the view SHALL show either the list or the selected category's details, the details offering a control back to the list. The selected category SHALL be part of the view's address, so the view can be linked to and reloaded with it selected. With no categories, the view SHALL invite the user to create one.

#### Scenario: Reaching the management view
- **WHEN** a user chooses "Categories" from the header Settings menu
- **THEN** the categories management view opens, listing every category with its number of members

#### Scenario: Renaming in place
- **WHEN** a user changes the selected category's name from "Chess" to "Board games" and moves focus away
- **THEN** the category is renamed to "Board games" in the view and the sidebar

#### Scenario: Rename rejected
- **WHEN** a user renames the selected category to the name of another category
- **THEN** the view says the name is already taken and the category keeps its saved name

#### Scenario: Members grouped by kind
- **WHEN** the selected category has the channels "Hikaru" and "GothamChess" and the playlist "Endgame course"
- **THEN** "In <name>" shows "GothamChess" and "Hikaru" under "Channels", then "Endgame course" under "Playlists"

#### Scenario: Removing a member
- **WHEN** a user activates "Remove" on a member
- **THEN** the source is no longer a member and appears in the add-members picker, without a confirmation

#### Scenario: Adding several members
- **WHEN** a user checks two channels and a playlist in the add-members picker and activates "Add selected"
- **THEN** all three become members of the category and leave the picker

#### Scenario: Filtering the picker
- **WHEN** a user types "ver" in the add-members search field
- **THEN** the picker lists only non-member sources whose name contains "ver", ignoring case

#### Scenario: Deleting from the management view
- **WHEN** a user deletes the selected category and confirms
- **THEN** the category is removed from the list and the sidebar, and no category is selected

#### Scenario: Narrow viewport
- **WHEN** a user on a mobile-width viewport selects a category in the management view
- **THEN** the view shows that category's details with a control back to the list

#### Scenario: Linking to a selected category
- **WHEN** a user reloads the management view while "Chess" is selected
- **THEN** the view opens with "Chess" selected

### Requirement: Category View
Each category SHALL have a view at an address of its own showing its name, the names of its members, and its videos as a grid of the same cards as the home view, each YouTube video once, newest sync first. The view SHALL offer "Mark all watched", which marks every downloaded video of the category watched without confirmation, and "Manage", which opens the categories management view with the category selected. A category with no members SHALL say so and link to the management view with it selected. A category without downloaded videos but with members SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

#### Scenario: Category videos
- **WHEN** a user opens the "Chess" view and its members have downloaded videos
- **THEN** the view shows "Chess", its member names, and the videos as cards, newest sync first

#### Scenario: Mark all watched
- **WHEN** a user activates "Mark all watched" on a category view
- **THEN** every card shows as watched, and the category's sidebar badge disappears promptly

#### Scenario: Empty category
- **WHEN** a user opens a category with no members
- **THEN** the view says it holds no channels or playlists yet and links to managing it

#### Scenario: Unknown category
- **WHEN** a user opens the view of a category that does not exist
- **THEN** the view says the category was not found

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

### Requirement: Header Settings Menu
The header SHALL offer, at its trailing edge, a settings control shown as a gear icon with the accessible name "Settings". Activating it SHALL open a menu whose items are navigation to other views. The menu SHALL contain a "Categories" item that navigates to the categories management view and a "Tasks" item that navigates to the tasks view. The header SHALL NOT show any other control for adding channels or playlists or for reaching the tasks view.

#### Scenario: Reaching the tasks view
- **WHEN** a user activates the header's settings control and chooses "Tasks"
- **THEN** the application navigates to the tasks view

#### Scenario: Reaching the categories management view
- **WHEN** a user activates the header's settings control and chooses "Categories"
- **THEN** the application navigates to the categories management view

#### Scenario: Header controls
- **WHEN** a user views any page
- **THEN** the header shows the settings control and no "Add" or "Tasks" button
