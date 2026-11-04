## MODIFIED Requirements

### Requirement: Mobile Layout Coverage
The suite SHALL verify on the iPhone browser that:
- no view (home, playlist, channel, category, tasks) scrolls horizontally
- the player stays directly below the header while the video list scrolls
- the video details start collapsed and "More" expands them
- the add, edit-settings and delete-confirm dialogs fit inside the viewport with their submit button reachable

#### Scenario: No horizontal overflow
- **WHEN** a user opens any view on a phone
- **THEN** the page is no wider than the viewport

#### Scenario: Sticky player
- **WHEN** a user scrolls a playlist's video list on a phone
- **THEN** the player's top edge stays at the header's bottom edge

#### Scenario: Collapsed video details
- **WHEN** a user opens a video on a phone
- **THEN** its storage location and the title row's "Open video on YouTube" link are visible, with no "More" control

#### Scenario: Dialogs fit the phone screen
- **WHEN** a user opens the Add playlist dialog (with the folder browser), the Edit settings dialog or the Delete confirmation on a phone
- **THEN** the dialog lies within the viewport and its submit button can be tapped to complete the action
