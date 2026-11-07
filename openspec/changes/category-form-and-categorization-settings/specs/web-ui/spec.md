## ADDED Requirements

### Requirement: Category Prompt Guide
A "?" control with the accessible name "About AI prompt" SHALL sit next to the "AI prompt" label and open, on click or tap, a guide to writing the prompt, with a good example, bad examples with the reason each is bad, and tips. Opening or closing it SHALL NOT stage a change or close the dialog.

#### Scenario: Opening the guide
- **WHEN** a user activates "About AI prompt"
- **THEN** the guide shows a good example saying what the video is about with "Belongs:" and "Does not belong:", bad examples with their reasons, and the tips: name what is close but doesn't belong, use words a title, chapter or summary would contain, and say whether one part of a video is enough

#### Scenario: Closing the guide
- **WHEN** a user with a staged change opens the guide and presses Escape
- **THEN** the guide closes, the dialog stays open and nothing is lost

## MODIFIED Requirements

### Requirement: Add Category Dialog
The "Add category" control SHALL open the category dialog, as "Category Settings Dialog" describes, titled "Add category", with an empty name, no sources and the palette color used by the fewest categories preselected (the earliest in palette order on a tie). Its save control SHALL read "Create", SHALL be possible once the name is not blank, and SHALL create the category with the chosen color, the text in "AI prompt" while it shows, and the pressed chips, in one request. On success, the dialog SHALL close and the application SHALL navigate to the new category's view. When creation fails (including a name already taken), the dialog SHALL stay open, keep everything entered and show the error. Dismissing it with anything entered SHALL ask to discard it first, saying "This category won't be created.". On viewports narrower than the desktop breakpoint, activating "Add category" SHALL close the sidebar before the dialog is shown.

#### Scenario: Creating a category
- **WHEN** a user activates "Add category", enters "Chess" and creates it
- **THEN** the dialog closes and the "Chess" category view opens, with no dialog open

#### Scenario: Next free color preselected
- **WHEN** a user opens "Add category" and the existing categories are colored Amber and Red
- **THEN** "Orange" is preselected

#### Scenario: Creating with a chosen color
- **WHEN** a user enters "Chess", chooses "Violet" and creates it
- **THEN** "Chess" is created with the color "#8b5cf6"

#### Scenario: Creating with everything in one go
- **WHEN** AI, categories and categorization are on and a user enters "Chess", writes an AI prompt, presses the chip "Hikaru" and creates it
- **THEN** one request creates "Chess" with that prompt and "Hikaru" as its source

#### Scenario: Creating without AI
- **WHEN** categorization is off and a user enters "Chess", presses a chip and creates it
- **THEN** there is no "AI prompt" field and "Chess" is created with that source and no AI prompt

#### Scenario: Discarding a new category
- **WHEN** a user enters "Chess" and cancels
- **THEN** the user is asked to discard it, and confirming closes the dialog and creates nothing

#### Scenario: Cancelling an untouched dialog
- **WHEN** a user opens "Add category" and cancels without entering anything
- **THEN** the dialog closes without asking

#### Scenario: Blank name
- **WHEN** the name field is empty or blank
- **THEN** the create action is disabled

#### Scenario: Name already taken
- **WHEN** a user creates a category with a name another category already has
- **THEN** the dialog stays open and says the name is already taken

### Requirement: Category View
Each category SHALL have a view at an address of its own with a header and its videos as a grid of the same cards as the home view, each YouTube video once, newest sync first. Activating a card SHALL open the video within the category, as described in "Category Playback", instead of in its channel's or playlist's view. The header SHALL be tinted with the soft tint of the category's color and SHALL show the category's name, a summary of its number of sources, its number of videos and, when above 0, its number of unwatched videos, and the same actions layout as the channel and playlist views' page header, offering "Edit settings", which opens the category settings dialog, "Mark all watched" (only when the category has sources), which marks every downloaded video of the category watched after the confirmation described in "Confirm Bulk Mark Watched", and "Delete", which deletes the category after the same confirmation as the sidebar row's "Delete" and then navigates to the home view; at or above the desktop breakpoint "Delete" SHALL be in the "More actions" menu as on the channel and playlist views. When the category is known from the categories list, the header SHALL show its name and tint at once, before its videos load, with a placeholder in place of the summary; the videos SHALL show as placeholders as described in "Loading Placeholders" until they load. A category with no sources SHALL say so and offer a control that opens the category settings dialog. A category with sources but without downloaded videos SHALL say it has no videos yet. When the category does not exist, the view SHALL say it was not found.

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
- **WHEN** a user chooses "Edit settings" on the "Chess" view, chooses "Violet" and saves
- **THEN** "Chess" is saved with the color "#8b5cf6", and the header, the sidebar dot and the home chips of its videos show it

#### Scenario: No separate color action
- **WHEN** a user opens the header actions of the "Chess" view
- **THEN** there is no "Change color" action

#### Scenario: Mark all watched
- **WHEN** a user chooses "Mark all watched" from a category view's header and confirms
- **THEN** every card shows as watched, and the category's sidebar count disappears promptly

#### Scenario: Deleting from the category view
- **WHEN** a user chooses "Delete" from the "Chess" view's header and confirms
- **THEN** the category is deleted and the application navigates to the home view

#### Scenario: Empty category
- **WHEN** a user opens a category with no sources
- **THEN** the view says it holds no channels or playlists yet and offers a control that opens the category settings dialog

#### Scenario: Unknown category
- **WHEN** a user opens the view of a category that does not exist
- **THEN** the view says the category was not found

### Requirement: Category Settings Dialog
The category dialog SHALL be titled "Edit <name> settings" when editing and SHALL show, in this order, a "Name" field, focused when the dialog opens, a "Color" category color picker, the AI prompt described below, and every tracked channel and playlist as a chip, channels under a "Channels" group and playlists under a "Playlists" group, each group ordered by name ignoring case. Channel chips SHALL show the channel's avatar; playlist chips a playlist icon. Each chip SHALL be a toggle whose pressed state means the source is in the category, shown with a check mark and a tinted fill, while a source not in it is shown outlined with a plus mark. Toggling a chip SHALL NOT change the order of the chips. A search field SHALL filter the chips by name ignoring case, and pressing Enter in it while exactly one chip matches SHALL toggle that chip; Enter in it SHALL never save. Pressing Enter in "Name" SHALL save. Filter controls "All", "In <name>" and "Not in <name>" ("In this category" and "Not in this category" while creating), the last two showing how many sources they hold, SHALL narrow the chips by state. A group with no chip left to show SHALL be hidden. While AI, categories and categorization are all on, the dialog SHALL show, above the search field, an "AI prompt" multi-line text field of 6 rows prefilled with the category's AI prompt, described by the hint "Describe the videos that belong in this category." and with the guide "Category Prompt Guide" describes; otherwise it SHALL NOT show, and the saved prompt SHALL be kept. Editing the name or color, toggling a chip or editing the prompt SHALL only stage changes: the dialog SHALL summarise the sources staged (such as "+2 added, −1 removed"), its "Save" control SHALL be disabled until something is staged and while the name is blank, and saving SHALL send only what changed, the name, color and AI prompt in one request, making the category's AI prompt the text in the field (blank removes it) and its sources exactly the pressed chips, then close the dialog. The AI prompt SHALL be sent only while its field shows and differs from what it started from, so a prompt changed elsewhere while the dialog is open is neither staged nor overwritten. The dialog SHALL set no length limit of its own: a prompt the daemon rejects SHALL show the daemon's message and keep the text. When saving fails, the dialog SHALL stay open, keep the staged changes and say why. When the name, color or AI prompt are saved but the sources fail, the error SHALL read "Your other changes were saved, but the sources weren't: <reason>", saving again SHALL send only the sources, and discarding SHALL say "Your other changes are already saved. Your changes to the sources in <name> won't be saved." Cancelling or dismissing the dialog with staged changes SHALL ask to discard them first. With no channel or playlist tracked, the dialog SHALL say so. Chips SHALL be reachable with the arrow keys within a group. On viewports narrower than the desktop breakpoint the dialog SHALL fill the screen; the name, color, AI prompt and chips SHALL scroll together, the search and filter controls SHALL stick to the top once scrolled to, and the save controls SHALL stay visible. At every viewport size the save controls SHALL stay visible, and the AI prompt field SHALL NOT be resizable.

#### Scenario: Name and color prefilled
- **WHEN** a user opens the dialog of "Chess", colored Violet
- **THEN** "Name" holds "Chess" and is focused, and "Violet" is shown checked

#### Scenario: Saving the name and color
- **WHEN** a user changes the name to "Board games", chooses "Teal" and saves
- **THEN** one update with the name and the color is sent, the members are not, and the dialog closes

#### Scenario: Blank name while editing
- **WHEN** a user empties "Name"
- **THEN** "Save" is disabled

#### Scenario: Name already taken while editing
- **WHEN** a user saves a name another category already has
- **THEN** the dialog stays open with everything entered and says the name is already taken, and the sources are not sent

#### Scenario: Enter in the search with several matches
- **WHEN** the search leaves several chips and the user presses Enter
- **THEN** nothing is toggled or saved

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
- **WHEN** nothing is changed, or every change has been undone
- **THEN** "Save" is disabled

#### Scenario: AI prompt shown
- **WHEN** AI, categories and categorization are on and a user opens the dialog of "Chess", whose AI prompt is "Chess games"
- **THEN** "AI prompt" holds "Chess games"

#### Scenario: AI prompt hidden
- **WHEN** AI or categorization is off and a user opens the dialog of "Chess"
- **THEN** there is no "AI prompt" field and saving sources leaves its AI prompt as it was

#### Scenario: Saving only the prompt
- **WHEN** a user changes "AI prompt" without toggling a chip and saves
- **THEN** only the category's `ai_prompt` is sent, its members are not, and the dialog closes

#### Scenario: Saving the prompt and sources
- **WHEN** a user changes "AI prompt", presses a chip and saves
- **THEN** both the prompt and the members are saved and the dialog closes

#### Scenario: Removing the prompt
- **WHEN** a user empties "AI prompt" of a category that has one and saves
- **THEN** the category has no AI prompt

#### Scenario: Prompt changed elsewhere
- **WHEN** the dialog of "Chess" is open, its AI prompt changes elsewhere and the user saves only a chip change
- **THEN** only the members are sent and the prompt saved elsewhere is kept

#### Scenario: Prompt too long
- **WHEN** a user pastes 2500 characters into "AI prompt" and saves
- **THEN** all 2500 are sent, and the dialog stays open with the text and shows the daemon's message

#### Scenario: Prompt saved but sources failed
- **WHEN** a user changes "AI prompt", presses a chip and saves, and only the sources fail
- **THEN** the dialog says "Your other changes were saved, but the sources weren't: <reason>", and saving again sends only the members

#### Scenario: Discarding after the prompt was saved
- **WHEN** the prompt was saved, the sources failed and the user cancels
- **THEN** the dialog asks to discard, saying "Your other changes are already saved. Your changes to the sources in Chess won't be saved."

#### Scenario: Discarding staged changes
- **WHEN** a user with staged changes cancels the dialog
- **THEN** the user is asked to discard them, and confirming closes the dialog without saving

#### Scenario: Save fails
- **WHEN** saving fails
- **THEN** the dialog stays open with the staged changes and says why

#### Scenario: Nothing tracked
- **WHEN** a user opens the dialog with no channel or playlist tracked
- **THEN** the dialog says there are no channels or playlists to add yet
