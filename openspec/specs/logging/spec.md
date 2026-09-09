# logging Specification

## Purpose

Defines how the daemon reports its own operational activity: at what
severity levels, how that verbosity is controlled at startup, and what
happens to that output in the automated test environment.

## Requirements

### Requirement: Leveled Log Output
The system SHALL categorize every operational message it emits (startup
checks, background sync/task/event activity, and fatal errors) at one of the
standard severity levels — error, warn, info, or debug — reflecting the
message's actual severity, and SHALL attach the relevant identifying values
(e.g. a playlist or task id) to the message as structured, queryable fields
rather than only interpolating them into free text.

#### Scenario: Routine activity is informational
- **WHEN** the daemon completes a routine operation (e.g. a successful
  startup check, a scheduled sync, a dispatched task)
- **THEN** the message is emitted at info level or below, not warn or error

#### Scenario: Failure is emitted at error level
- **WHEN** an operation fails in a way that prevents it from completing (e.g.
  a startup check fails, a background poll fails, a task or event is marked
  permanently failed)
- **THEN** the message is emitted at error level

#### Scenario: Identifying values are attached as fields
- **WHEN** a log message refers to a specific entity (a playlist, video,
  task, or event id)
- **THEN** that identifier is attached to the message as a named field, not
  only embedded in the message text

### Requirement: Configurable Verbosity at Startup
The system SHALL allow an operator to control which severity levels are
emitted by setting an environment variable when starting the daemon, without
requiring a rebuild, following the prevailing convention for the language
runtime the system is implemented in. The system SHALL emit info level and
above when the environment variable is not set.

#### Scenario: Operator raises verbosity
- **WHEN** the daemon is started with the log-level environment variable set
  to a more verbose level than the default (e.g. debug)
- **THEN** messages at that level and above are emitted

#### Scenario: Operator lowers verbosity
- **WHEN** the daemon is started with the log-level environment variable set
  to a less verbose level than the default (e.g. error only)
- **THEN** only messages at that level and above are emitted, and lower-
  severity messages (e.g. routine info-level activity) are suppressed

#### Scenario: No configuration provided
- **WHEN** the daemon is started without the log-level environment variable
  set
- **THEN** info-level and above messages are emitted, matching today's
  default visible output

### Requirement: No Log Output in Automated Tests
The system SHALL NOT emit log output during automated test runs, so test
output stays limited to test framework reporting (pass/fail, panics,
assertion failures).

#### Scenario: Test suite runs
- **WHEN** the automated test suite executes any test that exercises code
  paths which log
- **THEN** no log lines appear in the test run's output
