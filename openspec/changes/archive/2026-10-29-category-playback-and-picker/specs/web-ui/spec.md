## ADDED Requirements

### Requirement: Category Playback
A category view SHALL play its videos like a playlist view. Activating a card on the category view SHALL open that video within the category, at an address that identifies both the category and the video, with the video selected and autoplaying as for a deep-linked video. In this mode the view SHALL keep the category's header and SHALL show the player and the selected video's detail, the same as a channel or playlist view. The category's videos, in the order of the category's grid, SHALL be the video list beside them. Selecting a video in that list SHALL play it without leaving the category. Each video SHALL play from, offer the subtitles of, and record its watch progress against the channel or playlist the category lists it from. A video listed from a channel SHALL show that channel's avatar in its detail, linking to the channel. Opening the category without a video, for example from the sidebar, SHALL show the grid.

#### Scenario: Opening a card plays it in the category
- **WHEN** a user activates the card of the video "Rook endings", listed from the playlist "Endgames", on the "Chess" view
- **THEN** the address identifies the "Chess" category and "Rook endings", the player autoplays "Rook endings" from the playlist "Endgames", and the list beside it shows the videos of "Chess"

#### Scenario: Queue switches across sources
- **WHEN** a user playing "Rook endings" in the "Chess" view selects "Blitz", listed from the channel "Hikaru", in the list beside the player
- **THEN** the player plays "Blitz" from the channel "Hikaru", the view stays on "Chess", and the detail shows Hikaru's avatar linking to the channel

#### Scenario: Progress recorded against the source
- **WHEN** a user watches a video in a category view past the watched threshold
- **THEN** the video is watched in its channel or playlist and in the category, and the badges update as when watched from its source's view

#### Scenario: Back to the grid
- **WHEN** a user playing a video in the "Chess" view activates "Chess" in the sidebar
- **THEN** the view shows the grid of the category's videos

## MODIFIED Requirements

### Requirement: Category View
Each category SHALL have a view at an address of its own with a header and its videos as a grid of the same cards as the home view, each YouTube video once, newest sync first. Activating a card SHALL open the video within the category, as described in "Category Playback", instead of in its channel's or playlist's view. The header SHALL be tinted with the category's color and SHALL show the category's name, a color control with the accessible name and tooltip "Change color" that opens the category color picker and saves the chosen color immediately, a summary of its number of sources, its number of videos and, when above 0, its number of unwatched videos, and the same actions layout as the channel and playlist views' page header, offering "Manage sources", which opens the manage sources dialog, "Mark all watched" (only when the category has sources), which marks every downloaded video of the category watched after the confirmation described in "Confirm Bulk Mark Watched", and "Delete", which deletes the category after the same confirmation as the sidebar row's "Delete" and then navigates to the home view. A category with no sources SHALL say so and offer a control that opens the manage sources dialog. A category with sources but without downloaded videos SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

#### Scenario: Category videos
- **WHEN** a user opens the "Chess" view, which holds 2 sources with 14 downloaded videos, 3 of them unwatched
- **THEN** the view shows "Chess", the summary "2 sources · 14 videos · 3 unwatched", and the videos as cards, newest sync first

#### Scenario: Card opens the video in the category
- **WHEN** a user activates a video card on the "Chess" view
- **THEN** the application stays on the "Chess" view and plays that video, as described in "Category Playback"

#### Scenario: Header tinted
- **WHEN** a user opens the "Chess" view and "Chess" is colored "#d3ecf8"
- **THEN** the header's background is "#d3ecf8"

#### Scenario: Changing the color
- **WHEN** a user activates "Change color" on the "Chess" view and chooses "Lilac"
- **THEN** "Chess" is saved with the color "#e9def8", and the header, the sidebar dot and the home cards of its videos show it

#### Scenario: Mark all watched
- **WHEN** a user chooses "Mark all watched" from a category view's header and confirms
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

### Requirement: Categories Field In Add And Edit Dialogs
The add channel, add playlist, edit channel and edit playlist dialogs SHALL offer a "Categories" field showing every existing category as a toggle chip, ordered by name ignoring case. A chip's pressed state SHALL mean the source is chosen for that category. A chosen chip SHALL be filled with its category's color and show a check mark. A chip not chosen SHALL be outlined and show a dot in its category's color. Toggling a chip SHALL NOT change the order of the chips. The field SHALL also offer a "New category" text field with an "Add" control. Adding the typed name SHALL create a category with that name at once and choose it, or, when a category already has that name ignoring case, SHALL choose that category without creating one. Pressing Enter in the "New category" field SHALL add the typed name and SHALL NOT submit the dialog. Saving an add dialog SHALL track the source and then make it a member of exactly the chosen categories. Saving an edit dialog SHALL make the source a member of exactly the chosen categories, and a change of categories alone SHALL count as a change to save. When the source is saved but its categories cannot be, the dialog SHALL say so and keep the source saved.

#### Scenario: Every category shown
- **WHEN** a user opens the add channel dialog and the categories "Streams", "Chess" and "Science" exist
- **THEN** the Categories field shows the chips "Chess", "Science" and "Streams", none pressed, without typing anything

#### Scenario: Chosen chips in their category's color
- **WHEN** a user presses the "Chess" chip and "Chess" is colored "#d3ecf8"
- **THEN** the chip is pressed, filled with "#d3ecf8" and shows a check mark

#### Scenario: Choosing categories when adding a channel
- **WHEN** a user adds a channel with the "Chess" and "Streams" chips pressed
- **THEN** the channel is tracked and is a member of "Chess" and "Streams"

#### Scenario: Creating a category from the field
- **WHEN** a user types "Speedcubing", which matches no category, in "New category" and activates "Add"
- **THEN** the category "Speedcubing" exists and its chip is shown pressed

#### Scenario: Typing an existing name
- **WHEN** a user types "chess" in "New category" and presses Enter while "Chess" exists
- **THEN** the "Chess" chip is pressed, no category is created and the dialog is not submitted

#### Scenario: Editing a source's categories
- **WHEN** a user unpresses the "Chess" chip in a channel's edit dialog and saves
- **THEN** the channel is no longer a member of "Chess" and stays in its other categories

#### Scenario: Categories are the only change
- **WHEN** a user changes only the Categories field in an edit dialog
- **THEN** the dialog can be saved and saving updates the categories

#### Scenario: Categories fail to save
- **WHEN** a user adds a channel with categories chosen and the channel is tracked but its categories cannot be saved
- **THEN** the dialog says the channel was added but its categories were not saved

### Requirement: Home Card Category Tint
Each home video card SHALL take the colors of its categories, in the order the video lists them, at half opacity, as its background: none leaves the card without a tint, one gives a flat tint, two or three give equal diagonal bands with hard edges, in order from the top-left corner to the bottom-right one, the edge between two bands running from corner to corner whatever the card's shape, and four or more give a neutral grey, also at half opacity. Cards on a category's own view SHALL NOT be tinted.

#### Scenario: One category
- **WHEN** a home video belongs to "Chess", colored "#d3ecf8"
- **THEN** its card's background is "#d3ecf8" at 50% opacity

#### Scenario: Two categories
- **WHEN** a home video belongs to "Chess" ("#d3ecf8") and "Streams" ("#fff1bf")
- **THEN** its card is split diagonally, the "#d3ecf8" half first and the "#fff1bf" half second, both at 50% opacity

#### Scenario: Three categories
- **WHEN** a home video belongs to three categories
- **THEN** its card is split diagonally into three equal bands, one per category, in order

#### Scenario: Four or more categories
- **WHEN** a home video belongs to four categories
- **THEN** its card's background is neutral grey at 50% opacity

#### Scenario: No category
- **WHEN** a home video belongs to no category
- **THEN** its card has no tint

#### Scenario: Cards framed
- **WHEN** the home view shows tinted and untinted cards
- **THEN** every card has padding and a thin grey border, and neighbouring cards keep the grid's gap between them

#### Scenario: Category view cards
- **WHEN** a user opens the "Chess" view
- **THEN** its video cards have no tint
