# domain-events Specification

## Purpose

Provides a persisted, asynchronous mechanism for publishing domain events and dispatching each one to every independent subscriber registered for its type, so reacting to something that happened never blocks or risks the operation that caused it.

## Requirements

### Requirement: Durable Publication
The system SHALL persist an event before returning from the publish call, independent of whether or when it is ever consumed.

#### Scenario: Successful publish persists the event
- **WHEN** a domain event is published
- **THEN** the system records it in a pending state before the publish call returns

#### Scenario: Publish does not wait for processing
- **WHEN** a domain event is published
- **THEN** the publish call returns without waiting for any subscriber to run

### Requirement: Asynchronous Multi-Subscriber Dispatch
The system SHALL poll for pending events in the background and, for each one, invoke every subscriber currently registered for that event's type.

#### Scenario: One subscriber registered
- **WHEN** a pending event's type has exactly one registered subscriber
- **THEN** that subscriber is invoked with the event's payload

#### Scenario: Multiple subscribers registered
- **WHEN** a pending event's type has more than one registered subscriber
- **THEN** every registered subscriber is invoked

#### Scenario: No subscribers registered
- **WHEN** a pending event's type has no registered subscribers
- **THEN** the system marks the event processed without invoking anything and without treating it as an error

### Requirement: Retry Then Give Up
The system SHALL track a single retry counter per event. If any subscriber invocation for an event fails, the counter SHALL increment and the event SHALL remain eligible for another attempt, in which every subscriber for that event is invoked again. After 5 failed attempts, the event SHALL be marked permanently failed, logged, and not attempted again.

#### Scenario: A subscriber fails
- **WHEN** any subscriber invoked for an event raises an error
- **THEN** the event's retry counter increments and the event remains pending for another attempt, in which all of its subscribers are invoked again, including ones that already succeeded on a prior attempt

#### Scenario: Fifth consecutive failure
- **WHEN** an event's retry counter reaches 5 failed attempts
- **THEN** the system logs the failure, marks the event permanently failed, and does not attempt it again

### Requirement: Isolation from the Publisher
The system SHALL ensure that a failure or delay in processing an event never affects the outcome already returned by the operation that published it.

#### Scenario: Subscriber fails after publish already returned
- **WHEN** a subscriber invocation for an event fails
- **THEN** the operation that published the event is unaffected, since it already completed before the event was processed
