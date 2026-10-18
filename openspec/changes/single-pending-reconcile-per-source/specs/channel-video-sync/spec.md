## MODIFIED Requirements

### Requirement: Recurring Channel Reconciliation
The system SHALL schedule the next reconcile of a channel after every reconcile attempt that runs to completion, after the channel's own reconcile interval as stored when the attempt finishes, regardless of whether any videos were added or evicted. When the channel no longer exists when the attempt finishes, the system SHALL schedule no further reconcile of it. A channel SHALL have at most one pending recurring reconcile: when a reconcile of the channel is already pending when the attempt finishes, the system SHALL schedule no further one.

#### Scenario: Reconcile finds no changes
- **WHEN** a channel reconcile completes and finds no membership changes
- **THEN** the system still schedules the next reconcile of that channel after the channel's reconcile interval

#### Scenario: Reconcile finds changes
- **WHEN** a channel reconcile completes and finds videos added and/or evicted
- **THEN** the system schedules the next reconcile of that channel after the channel's reconcile interval, in addition to persisting the changes

#### Scenario: Channels reconcile on their own intervals
- **WHEN** a channel with a 30 minute interval and a channel with a 1440 minute interval each complete a reconcile at the same time
- **THEN** the next reconcile of the first is scheduled 30 minutes later and of the second 1440 minutes later

#### Scenario: Interval changed during a reconcile
- **WHEN** a channel's interval is changed from 180 to 60 minutes while a reconcile of it is running
- **THEN** that reconcile, when it completes, schedules the next one 60 minutes later

#### Scenario: Channel deleted during a reconcile
- **WHEN** a channel is deleted while a reconcile of it is running
- **THEN** that reconcile, when it completes, schedules no further reconcile of the channel

#### Scenario: Reconcile already pending
- **WHEN** a reconcile of a channel completes while another reconcile of the same channel is already pending
- **THEN** the system schedules no further reconcile, and the pending one keeps its time

#### Scenario: Parallel reconcile chains converge
- **WHEN** a channel has several pending reconciles, left by a version without this rule
- **THEN** each extra one, when it completes while another is pending, schedules no successor, until the channel has a single pending reconcile
