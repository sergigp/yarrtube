## ADDED Requirements

### Requirement: Subscriber Panic Counts As A Failure
The system SHALL treat a subscriber that panics while handling an event as a failed invocation, the same as one that returns an error: the event's retry counter increments, the event follows the retry and dead-letter rules, and the panic's message is recorded as the event's error. A panic MUST NOT stop the consumer from processing the other pending events.

#### Scenario: A subscriber panics
- **WHEN** a subscriber invoked for an event panics
- **THEN** the event's retry counter increments and its error records that the subscriber panicked, with the panic message

#### Scenario: A subscriber panics on the fifth attempt
- **WHEN** a subscriber panics on an event's 5th attempt
- **THEN** the event is moved to the dead-letter table and deleted from the events table

#### Scenario: Other events still processed after a panic
- **WHEN** a subscriber panics on one pending event and other events are pending in the same poll
- **THEN** the other events are dispatched to their subscribers in that same poll
