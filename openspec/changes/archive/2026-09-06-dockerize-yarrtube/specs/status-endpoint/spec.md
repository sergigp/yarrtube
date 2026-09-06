## Purpose

Provides a minimal HTTP endpoint that confirms the running yarrtube daemon is reachable, as the first surface of the REST API the project will grow into.

## ADDED Requirements

### Requirement: Status Endpoint
The system SHALL expose an HTTP `GET /status` endpoint on the daemon's HTTP server that responds with a `200 OK` status whenever the server is running, without requiring authentication.

#### Scenario: Daemon is running
- **WHEN** a client sends `GET /status` to the running daemon
- **THEN** the daemon responds with HTTP status 200
