# Proposal

## Why

An audit of `add-preferences` (merged in #126) found two web defects. Two quick toggles on the preferences view could undo each other: each save sends every preference, built from values that a save still in flight hasn't updated, so the daemon could revert a change while the view said "Saved". And after a preferences change, home briefly showed sections from its cached response, including category sections while categories are off.

## What Changes

- The preferences view saves one preference at a time: every switch is disabled while a save is in flight.
- Saving preferences reloads the cached home videos at once, and home never renders category sections while categories are off, whatever its cached response holds.
- Audit cleanups with no behaviour change: the "Saved" timer hook moves to `web/src/hooks/useSavedFlash.ts`, tests use `queryKeys`, a leftover comment and an import order are fixed.

## Capabilities

### New Capabilities

### Modified Capabilities
- `web-ui`: preferences are saved one at a time; home reflects a preferences change at once.

## Impact

Web only: `web/src/components/PreferencesView.tsx`, `web/src/components/Home.tsx`, `web/src/api/queries.ts`, `web/src/hooks/useSavedFlash.ts` and their tests. No API or storage change.
