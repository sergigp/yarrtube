## ADDED Requirements

### Requirement: Plex Requests Recover Without A Restart
When the integration is enabled, a Plex request that fails because of the HTTP client itself (not because of the Plex server's reply) SHALL affect only that request. Later Plex requests MUST be attempted afresh and succeed when the Plex server is reachable, without restarting the daemon.

#### Scenario: Later request after a client failure
- **WHEN** a Plex request fails because the HTTP client broke, and a later Plex request is made while the server is reachable
- **THEN** the later request reaches the Plex server and succeeds
