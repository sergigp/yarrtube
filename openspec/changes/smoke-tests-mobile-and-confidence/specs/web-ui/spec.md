## ADDED Requirements

### Requirement: Accessible Sidebar Section Lists
Each sidebar section's list of entries (Channels, Playlists, Categories) SHALL be exposed to assistive technology as a list named after its section heading.

#### Scenario: Channels list is named
- **WHEN** at least one channel is tracked and the Channels section is unfolded
- **THEN** its entries are in a list whose accessible name is "Channels"

### Requirement: Add Dialog Submit Errors Are Announced
When creating a channel or playlist from the Add dialog fails, the dialog SHALL present the error message as an alert so assistive technology announces it.

#### Scenario: Create fails
- **WHEN** the server rejects a playlist created from the Add dialog
- **THEN** the dialog shows the server's message in an element with the alert role
