# directory-browsing Specification

## Purpose

Lets a client enumerate the directories that exist under the configured videos root, one level at a time, so that a storage location can be chosen by browsing what is actually on disk rather than typed from memory.

## Requirements

### Requirement: List Directories Under The Videos Root
The system SHALL provide an HTTP endpoint that returns the immediate subdirectories of a directory identified by a path relative to the configured videos root. Omitting the path SHALL list the videos root itself.

The response SHALL identify the directory that was listed and SHALL name each immediate subdirectory. It SHALL contain directories only; regular files SHALL be omitted, because the caller is choosing a storage location and a file can never be one. Entries SHALL be ordered deterministically so that repeated requests for an unchanged directory return them in the same order.

Every response SHALL also report the configured videos root as an absolute path. The videos root is deployment-specific and no other endpoint exposes it, so without it a caller cannot present the absolute location a chosen directory corresponds to. Reporting it on every listing rather than only on some keeps a caller from having to order one request before another.

The endpoint SHALL descend exactly one level per request. It SHALL NOT walk the tree recursively: videos are stored one directory per video, so a recursive listing of the share would be dominated by per-video directories that are never valid choices.

#### Scenario: Listing the videos root
- **WHEN** a client requests a listing without supplying a path
- **THEN** the system returns the immediate subdirectories of the configured videos root

#### Scenario: Response reports the videos root
- **WHEN** a client requests any successful listing
- **THEN** the response reports the configured videos root as an absolute path, whether the listed directory is the root itself or nested below it

#### Scenario: Listing a nested directory
- **WHEN** a client requests a listing for a relative path that identifies an existing directory under the videos root
- **THEN** the system returns that directory's immediate subdirectories, and no entry from any deeper level

#### Scenario: Directory containing both files and subdirectories
- **WHEN** a client lists a directory that contains both regular files and subdirectories
- **THEN** the response names the subdirectories and omits the regular files

#### Scenario: Empty directory
- **WHEN** a client lists an existing directory that has no subdirectories
- **THEN** the system succeeds and returns an empty list of entries, not an error

#### Scenario: Hidden directory
- **WHEN** a client lists a directory that contains a subdirectory whose name begins with a dot
- **THEN** that subdirectory is omitted from the response

#### Scenario: Nonexistent directory
- **WHEN** a client requests a listing for a relative path that does not exist under the videos root
- **THEN** the system returns a not-found response with a meaningful error description

#### Scenario: Path identifying a regular file
- **WHEN** a client requests a listing for a relative path that exists but is a regular file rather than a directory
- **THEN** the system rejects the request with a meaningful error description

### Requirement: Confinement To The Videos Root
The system SHALL resolve every requested path against the configured videos root and SHALL refuse any request whose resolved target lies outside that root. Confinement SHALL be enforced against the fully resolved location, so that a symbolic link inside the videos root cannot be used to enumerate a directory outside it.

This endpoint requires no authentication, as with every other route this server exposes, so confinement is the only control preventing arbitrary directories on the host from being enumerated.

#### Scenario: Path containing a parent traversal segment
- **WHEN** a client requests a listing for a relative path containing a `..` segment
- **THEN** the system rejects the request with a meaningful error description and discloses nothing about any directory outside the videos root

#### Scenario: Absolute path
- **WHEN** a client requests a listing for an absolute path
- **THEN** the system rejects the request with a meaningful error description and discloses nothing about any directory outside the videos root

#### Scenario: Symbolic link pointing outside the videos root
- **WHEN** a client requests a listing for a path that reaches a symbolic link inside the videos root whose target resolves outside that root
- **THEN** the system rejects the request with a meaningful error description rather than listing the link's target

#### Scenario: Symbolic link pointing inside the videos root
- **WHEN** a client requests a listing for a path that reaches a symbolic link inside the videos root whose target also resolves within that root
- **THEN** the system lists the target directory's immediate subdirectories
