# Spec Delta

## MODIFIED Requirements

### Requirement: Watch Sync In The Tasks View
The tasks view SHALL describe a pending watch sync as "Sync watch state with Plex" and a running one as "Syncing watch state with Plex", under the **Plex & upkeep** chip.

#### Scenario: Watch sync pending
- **WHEN** a watch sync is pending
- **THEN** the tasks view lists it as "Sync watch state with Plex" under **Plex & upkeep**

#### Scenario: Watch sync running
- **WHEN** a watch sync is running
- **THEN** the tasks view lists it under **Running now** as "Syncing watch state with Plex"
