# Spec Delta

## MODIFIED Requirements

### Requirement: Retry Then Give Up
The system SHALL track a single retry counter per event. If any subscriber invocation for an event fails, the counter SHALL increment and the event SHALL become eligible for another attempt only after a retry delay, in which every subscriber for that event is invoked again. After 5 failed attempts, the system SHALL log the failure, move the event to the dead-letter table, and remove it from the events table so it is not attempted again.

#### Scenario: A subscriber fails
- **WHEN** any subscriber invoked for an event raises an error
- **THEN** the event's retry counter increments and the event remains pending for another attempt, in which all of its subscribers are invoked again, including ones that already succeeded on a prior attempt

#### Scenario: Fifth consecutive failure
- **WHEN** an event's retry counter reaches 5 failed attempts
- **THEN** the system logs the failure, records the event's id, type, payload, final error, attempt count, and timestamps in the dead-letter table, and deletes the event from the events table

## ADDED Requirements

### Requirement: Growing Delay Between Event Attempts
After a failed attempt, an event SHALL NOT be attempted again until `base * 3^retries` seconds have passed, where `retries` is its retry counter after the failure and `base` is the same configurable base retry delay tasks use (`YARRTUBE_RETRY_BASE_DELAY_SECONDS`, default 150 s). A newly published event SHALL be eligible immediately.

#### Scenario: Not retried before its delay
- **WHEN** an event's first attempt fails and the consumer polls again before `base * 3` seconds have passed
- **THEN** the event is not dispatched in that poll and stays pending with its retry counter at 1

#### Scenario: Retried once its delay has passed
- **WHEN** an event's first attempt failed and the consumer polls once `base * 3` seconds have passed
- **THEN** the event is dispatched to all of its subscribers again

#### Scenario: Delay grows with each failure
- **WHEN** an event that has already failed `n` times fails again
- **THEN** it becomes eligible again `base * 3^(n+1)` seconds after that failure

#### Scenario: Newly published event
- **WHEN** an event is published
- **THEN** it is dispatched on the next poll

#### Scenario: Events pending before the upgrade
- **WHEN** the daemon starts on a database holding pending events written by a version without retry delays
- **THEN** those events are dispatched on the next poll, keeping their retry counters

### Requirement: Eligible Events Processed In Publication Order
Each poll SHALL dispatch the events whose retry delay has passed in the order they were published. An event waiting out its retry delay SHALL NOT hold back later events, including later events about the same playlist, channel or video.

#### Scenario: Eligible events in order
- **WHEN** several events are eligible in the same poll
- **THEN** they are dispatched in the order they were published

#### Scenario: A backing-off event is overtaken
- **WHEN** an earlier event is waiting out its retry delay and a later event is eligible
- **THEN** the later event is dispatched and completed without waiting for the earlier one

### Requirement: A Backing-Off Event Is Still Pending
An event waiting out its retry delay SHALL still count as pending for everything that reacts to pending events other than dispatch, so it keeps holding back what a not-yet-handled event holds back until it is handled or dead-lettered.

#### Scenario: A backing-off deletion still reserves its folder
- **WHEN** a playlist or channel deletion event is waiting out its retry delay and a show is moved into that source's folder
- **THEN** the move is rejected as a move into the folder of a source whose deletion is still pending

#### Scenario: A backing-off download keeps its show unsettled
- **WHEN** a video's downloaded event is waiting out its retry delay
- **THEN** its show is listed as having work in progress and cannot be moved

### Requirement: Event Error Records The Full Cause
The error the system stores for a failed event, on the pending event and on its dead-letter record, SHALL name the failure's full cause chain on one line (outermost context first, root cause last), not only its outermost message.

#### Scenario: A subscriber fails with a nested cause
- **WHEN** a subscriber fails with "error sending request for url (…)" caused by "Connection refused"
- **THEN** the event's stored error is "error sending request for url (…): Connection refused"

#### Scenario: Dead letter keeps the full cause
- **WHEN** an event is dead-lettered after a subscriber failed with a nested cause
- **THEN** the dead-letter record's final error is the full cause chain
