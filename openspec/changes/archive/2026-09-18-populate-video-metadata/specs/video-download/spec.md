## REMOVED Requirements

### Requirement: Metadata File Placeholder
**Reason**: Superseded by the `video-metadata` capability, which generates
real, populated `movie.nfo` content immediately after a successful download
instead of an empty placeholder file (and under a different filename).
**Migration**: No action needed — a successful video download no longer
creates a placeholder metadata file on its own; see `video-metadata` for
the file it now creates instead.
