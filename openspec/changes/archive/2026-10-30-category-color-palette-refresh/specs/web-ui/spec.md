# Spec Delta

## MODIFIED Requirements

### Requirement: Add Category Dialog
The "Add category" control SHALL open a dialog asking for a name and a color through the category color picker, with the palette color used by the fewest categories preselected (the earliest in palette order on a tie). Creating SHALL be possible once the name is not blank, and SHALL create the category with the chosen color. On success, the dialog SHALL close and the application SHALL navigate to the new category's view with the manage sources dialog open. When creation fails (including a name already taken), the dialog SHALL stay open and show the error. On viewports narrower than the desktop breakpoint, activating "Add category" SHALL close the sidebar before the dialog is shown.

#### Scenario: Creating a category
- **WHEN** a user activates "Add category", enters "Chess" and creates it
- **THEN** the dialog closes and the "Chess" category view opens with the manage sources dialog open and its search field focused

#### Scenario: Next free color preselected
- **WHEN** a user opens "Add category" and the existing categories are colored Amber and Red
- **THEN** "Orange" is preselected

#### Scenario: Creating with a chosen color
- **WHEN** a user enters "Chess", chooses "Violet" and creates it
- **THEN** "Chess" is created with the color "#8b5cf6"

#### Scenario: Blank name
- **WHEN** the name field is empty or blank
- **THEN** the create action is disabled

#### Scenario: Name already taken
- **WHEN** a user creates a category with a name another category already has
- **THEN** the dialog stays open and says the name is already taken

### Requirement: Category View
Each category SHALL have a view at an address of its own with a header and its videos as a grid of the same cards as the home view, each YouTube video once, newest sync first. Activating a card SHALL open the video within the category, as described in "Category Playback", instead of in its channel's or playlist's view. The header SHALL be tinted with the soft tint of the category's color and SHALL show the category's name, a summary of its number of sources, its number of videos and, when above 0, its number of unwatched videos, and the same actions layout as the channel and playlist views' page header, offering "Change color", which opens the category color picker and saves the chosen color immediately, "Manage sources", which opens the manage sources dialog, "Mark all watched" (only when the category has sources), which marks every downloaded video of the category watched after the confirmation described in "Confirm Bulk Mark Watched", and "Delete", which deletes the category after the same confirmation as the sidebar row's "Delete" and then navigates to the home view; at or above the desktop breakpoint "Delete" SHALL be in the "More actions" menu as on the channel and playlist views. When the category is known from the categories list, the header SHALL show its name and tint at once, before its videos load, with a placeholder in place of the summary; the videos SHALL show as placeholders as described in "Loading Placeholders" until they load. A category with no sources SHALL say so and offer a control that opens the manage sources dialog. A category with sources but without downloaded videos SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

#### Scenario: Category videos
- **WHEN** a user opens the "Chess" view, which holds 2 sources with 14 downloaded videos, 3 of them unwatched
- **THEN** the view shows "Chess", the summary "2 sources · 14 videos · 3 unwatched", and the videos as cards, newest sync first

#### Scenario: Card opens the video in the category
- **WHEN** a user activates a video card on the "Chess" view
- **THEN** the application stays on the "Chess" view and plays that video, as described in "Category Playback"

#### Scenario: Header tinted
- **WHEN** a user opens the "Chess" view and "Chess" is colored "#0ea5e9"
- **THEN** the header's background is the soft tint of that color's hue

#### Scenario: Header while loading
- **WHEN** a user opens the "Chess" view from the sidebar and its videos are still loading
- **THEN** the header already shows "Chess" with its tint and actions, a placeholder in place of the summary, and placeholder cards below

#### Scenario: Changing the color
- **WHEN** a user activates "Change color" on the "Chess" view and chooses "Violet"
- **THEN** "Chess" is saved with the color "#8b5cf6", and the header, the sidebar dot and the home chips of its videos show it

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

### Requirement: Categories Field In Add And Edit Dialogs
The add channel, add playlist, edit channel and edit playlist dialogs SHALL offer a "Categories" field showing every existing category as a toggle chip, ordered by name ignoring case. A chip's pressed state SHALL mean the source is chosen for that category. A chosen chip SHALL be filled with the soft tint of its category's hue, show its name in the dark shade of that hue, as the home card chips do, and show a check mark. A chip not chosen SHALL be outlined and show a dot in its category's color. Toggling a chip SHALL NOT change the order of the chips. The field SHALL also offer a "New category" text field with an "Add" control. Adding the typed name SHALL create a category with that name at once and choose it, or, when a category already has that name ignoring case, SHALL choose that category without creating one. Pressing Enter in the "New category" field SHALL add the typed name and SHALL NOT submit the dialog. Saving an add dialog SHALL track the source and then make it a member of exactly the chosen categories. Saving an edit dialog SHALL make the source a member of exactly the chosen categories, and a change of categories alone SHALL count as a change to save. When the source is saved but its categories cannot be, the dialog SHALL say so and keep the source saved.

#### Scenario: Every category shown
- **WHEN** a user opens the add channel dialog and the categories "Streams", "Chess" and "Science" exist
- **THEN** the Categories field shows the chips "Chess", "Science" and "Streams", none pressed, without typing anything

#### Scenario: Chosen chips in their category's color
- **WHEN** a user presses the "Chess" chip and "Chess" is colored "#0ea5e9"
- **THEN** the chip is pressed, filled with the soft tint of that color's hue, its name in the dark shade of that hue, and shows a check mark

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

### Requirement: Category Color Picker
The category color picker SHALL offer the 10 palette colors as swatches in palette order, each a radio option named Amber, Orange, Red, Pink, Violet, Blue, Sky, Teal, Green and Slate, plus a "Custom" option with a hue slider. Every custom color SHALL have OKLCH lightness 0.65 and chroma 0.2 (chroma lowered as needed to fit sRGB), so only the hue changes. The chosen option SHALL be shown checked. "Custom" SHALL stay chosen, with its slider shown, until a swatch is chosen, even when the slider lands on a palette color. A color not in the palette SHALL show as "Custom" with the slider at its hue.

#### Scenario: Choosing a swatch
- **WHEN** a user chooses the "Sky" swatch
- **THEN** "Sky" is shown checked and the chosen color is "#0ea5e9"

#### Scenario: Clicking a swatch
- **WHEN** a user clicks anywhere on the "Violet" swatch
- **THEN** "Violet" is shown checked

#### Scenario: Choosing a custom hue
- **WHEN** a user chooses "Custom" and moves the hue slider
- **THEN** the chosen color follows the hue at OKLCH lightness 0.65 and chroma 0.2

#### Scenario: Choosing Custom from any swatch
- **WHEN** the "Orange" or "Blue" swatch is checked and a user chooses "Custom"
- **THEN** "Custom" is shown checked and the hue slider is shown

#### Scenario: Custom hue matching a palette color
- **WHEN** "Custom" is chosen and the user moves the slider to a hue whose color equals a palette color
- **THEN** "Custom" stays checked and the slider stays shown

#### Scenario: Swatch after Custom
- **WHEN** "Custom" is chosen and the user chooses the "Teal" swatch
- **THEN** "Teal" is shown checked and the hue slider is hidden

#### Scenario: Existing custom color
- **WHEN** the picker opens for a category whose color is not in the palette
- **THEN** "Custom" is shown checked with the slider at that color's hue
