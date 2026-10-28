## ADDED Requirements

### Requirement: Category Color Picker
The category color picker SHALL offer the 10 palette colors as swatches in palette order, each a radio option named Butter, Peach, Coral, Rose, Lilac, Periwinkle, Sky, Aqua, Pistachio and Sand, plus a "Custom" option with a hue slider. Every custom color SHALL keep the palette's soft lightness and saturation, so only the hue changes. The chosen option SHALL be shown checked. A color not in the palette SHALL show as "Custom" with the slider at its hue.

#### Scenario: Choosing a swatch
- **WHEN** a user chooses the "Sky" swatch
- **THEN** "Sky" is shown checked and the chosen color is "#d3ecf8"

#### Scenario: Clicking a swatch
- **WHEN** a user clicks anywhere on the "Lilac" swatch
- **THEN** "Lilac" is shown checked

#### Scenario: Choosing a custom hue
- **WHEN** a user chooses "Custom" and moves the hue slider
- **THEN** the chosen color follows the hue and stays a soft pastel

#### Scenario: Existing custom color
- **WHEN** the picker opens for a category whose color is not in the palette
- **THEN** "Custom" is shown checked with the slider at that color's hue

### Requirement: Home Card Category Tint
Each home video card SHALL take the colors of its categories, in the order the video lists them, as its background: none leaves the card without a tint, one gives a flat tint, two or three give equal diagonal bands with hard edges, in order from the top-left corner to the bottom-right one, the edge between two bands running from corner to corner whatever the card's shape, and four or more give a neutral grey. Cards on a category's own view SHALL NOT be tinted.

#### Scenario: One category
- **WHEN** a home video belongs to "Chess", colored "#d3ecf8"
- **THEN** its card's background is "#d3ecf8"

#### Scenario: Two categories
- **WHEN** a home video belongs to "Chess" ("#d3ecf8") and "Streams" ("#fff1bf")
- **THEN** its card is split diagonally, the "#d3ecf8" half first and the "#fff1bf" half second

#### Scenario: Three categories
- **WHEN** a home video belongs to three categories
- **THEN** its card is split diagonally into three equal bands, one per category, in order

#### Scenario: Four or more categories
- **WHEN** a home video belongs to four categories
- **THEN** its card's background is neutral grey

#### Scenario: No category
- **WHEN** a home video belongs to no category
- **THEN** its card has no tint

#### Scenario: Cards framed
- **WHEN** the home view shows tinted and untinted cards
- **THEN** every card has padding and a thin grey border, and neighbouring cards keep the grid's gap between them

#### Scenario: Category view cards
- **WHEN** a user opens the "Chess" view
- **THEN** its video cards have no tint

## MODIFIED Requirements

### Requirement: Add Category Dialog
The "Add category" control SHALL open a dialog asking for a name and a color through the category color picker, with the palette color used by the fewest categories preselected (the earliest in palette order on a tie). Creating SHALL be possible once the name is not blank, and SHALL create the category with the chosen color. On success, the dialog SHALL close and the application SHALL navigate to the new category's view with the manage sources dialog open. When creation fails (including a name already taken), the dialog SHALL stay open and show the error. On viewports narrower than the desktop breakpoint, activating "Add category" SHALL close the sidebar before the dialog is shown.

#### Scenario: Creating a category
- **WHEN** a user activates "Add category", enters "Chess" and creates it
- **THEN** the dialog closes and the "Chess" category view opens with the manage sources dialog open and its search field focused

#### Scenario: Next free color preselected
- **WHEN** a user opens "Add category" and the existing categories are colored Butter and Coral
- **THEN** "Peach" is preselected

#### Scenario: Creating with a chosen color
- **WHEN** a user enters "Chess", chooses "Lilac" and creates it
- **THEN** "Chess" is created with the color "#e9def8"

#### Scenario: Blank name
- **WHEN** the name field is empty or blank
- **THEN** the create action is disabled

#### Scenario: Name already taken
- **WHEN** a user creates a category with a name another category already has
- **THEN** the dialog stays open and says the name is already taken

### Requirement: Category View
Each category SHALL have a view at an address of its own with a header and its videos as a grid of the same cards as the home view, each YouTube video once, newest sync first. The header SHALL be tinted with the category's color and SHALL show the category's name, a color control with the accessible name and tooltip "Change color" that opens the category color picker and saves the chosen color immediately, a summary of its number of sources, its number of videos and, when above 0, its number of unwatched videos, and the same actions layout as the channel and playlist views' page header, offering "Manage sources", which opens the manage sources dialog, "Mark all watched" (only when the category has sources), which marks every downloaded video of the category watched after the confirmation described in "Confirm Bulk Mark Watched", and "Delete", which deletes the category after the same confirmation as the sidebar row's "Delete" and then navigates to the home view. A category with no sources SHALL say so and offer a control that opens the manage sources dialog. A category with sources but without downloaded videos SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

#### Scenario: Category videos
- **WHEN** a user opens the "Chess" view, which holds 2 sources with 14 downloaded videos, 3 of them unwatched
- **THEN** the view shows "Chess", the summary "2 sources · 14 videos · 3 unwatched", and the videos as cards, newest sync first

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

### Requirement: Sidebar Categories List
The sidebar SHALL show a "Categories" section above the Channels section listing every category, never trimmed, each row showing a dot in the category's color, the category's name and, when its unwatched count is above 0, a badge with that count. Selecting a row SHALL navigate to that category's view, and while the user is on it, its row SHALL be marked active. The sidebar search SHALL filter the section like the other sections.

#### Scenario: Categories listed above channels
- **WHEN** the categories "Chess" and "Sports" exist
- **THEN** the sidebar shows a "Categories" section above "Channels" listing both

#### Scenario: Color dot
- **WHEN** the category "Chess" is colored "#d3ecf8"
- **THEN** its row shows a "#d3ecf8" dot before its name

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
