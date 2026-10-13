## ADDED Requirements

### Requirement: Channel Avatar Copied As Show Poster
The system SHALL copy a channel's stored avatar into the channel's show directory as `poster.jpg` whenever it writes the channel's show files (on creation and on every reconcile pass, see `tv-show-layout`), overwriting any existing poster so a changed avatar propagates. A channel with no stored avatar SHALL get no poster, and an existing `poster.jpg` SHALL then be left as it is.

#### Scenario: Channel with a stored avatar
- **WHEN** a channel with a recorded avatar filename has its show files written
- **THEN** its show directory contains a `poster.jpg` with the avatar's bytes

#### Scenario: Channel without a stored avatar
- **WHEN** a channel with no recorded avatar filename has its show files written
- **THEN** no `poster.jpg` is written and an existing one is left untouched
