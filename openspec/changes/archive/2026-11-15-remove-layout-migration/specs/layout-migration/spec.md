## REMOVED Requirements

### Requirement: Migrate-Layout Subcommand
**Reason**: The 0.9.0 layout migration was a one-off; 1.0 drops it.
**Migration**: A library still in the per-video-folder layout must first run `migrate-layout` with a 0.9.x or 0.10.x image, then upgrade.

### Requirement: Dry Run
**Reason**: Part of the removed `migrate-layout` subcommand.
**Migration**: None; the subcommand is gone.

### Requirement: Videos The Migration Cannot Place Are Skipped
**Reason**: Part of the removed `migrate-layout` subcommand.
**Migration**: None; the subcommand is gone.

### Requirement: Migration Is Idempotent And Resumable
**Reason**: Part of the removed `migrate-layout` subcommand.
**Migration**: None; the subcommand is gone.
