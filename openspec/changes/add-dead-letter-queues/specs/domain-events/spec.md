## MODIFIED Requirements

### Requirement: Retry Then Give Up
The system SHALL track a single retry counter per event. If any subscriber invocation for an event fails, the counter SHALL increment and the event SHALL remain eligible for another attempt, in which every subscriber for that event is invoked again. After 5 failed attempts, the system SHALL log the failure, move the event to the dead-letter table, and remove it from the events table so it is not attempted again.

#### Scenario: A subscriber fails
- **WHEN** any subscriber invoked for an event raises an error
- **THEN** the event's retry counter increments and the event remains pending for another attempt, in which all of its subscribers are invoked again, including ones that already succeeded on a prior attempt

#### Scenario: Fifth consecutive failure
- **WHEN** an event's retry counter reaches 5 failed attempts
- **THEN** the system logs the failure, records the event's id, type, payload, final error, attempt count, and timestamps in the dead-letter table, and deletes the event from the events table

## ADDED Requirements

### Requirement: Dead-Letter Record for Permanently Failed Events
The system SHALL persist a durable record of every event that exhausts its retries, so a permanently failed event remains inspectable after it leaves the events table.

#### Scenario: Event moved to dead letter
- **WHEN** an event reaches its 5th failed attempt
- **THEN** a record containing the event's original id, type, payload, final error message, number of attempts, and timestamps is inserted into the dead-letter table

### Requirement: Events Table Behaves as a Queue
The system SHALL remove an event from the events table as soon as it reaches a terminal state, so the table only ever contains events that are still pending or eligible for retry.

#### Scenario: Event processed successfully
- **WHEN** every subscriber for an event succeeds
- **THEN** the event is deleted from the events table rather than being retained with a completed status

#### Scenario: Event exhausts retries
- **WHEN** an event reaches its 5th failed attempt and is moved to the dead-letter table
- **THEN** it is also deleted from the events table
