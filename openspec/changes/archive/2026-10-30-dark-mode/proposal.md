## Why

Yarrtube is often used in the evening on a TV-adjacent laptop or phone, and its all-white UI is glaring there. A dark mode, chosen once and kept on the daemon, makes it comfortable everywhere without setting it per browser.

## What Changes

- New `theme` preference (`"light"` or `"dark"`, default `"light"`) read and saved through `GET`/`PUT /api/preferences`, validated like the other preferences.
- A "Dark mode" switch in the header, left of the settings gear: a round knob that slides across, showing a sun in light mode and a moon in dark mode.
- The whole web UI gets a dark palette, drawn as soon as the switch flips and saved as the preference; a failed save draws the saved theme back.
- The last drawn theme is cached in the browser only so a reload paints it before the preferences arrive; the saved preference stays the source of truth.
- Category chips and the category view header use a dark-mode tint and a light ink of the category's hue, readable at 4.5:1 or more.
- Video list rows are separated by straight lines instead of bottom borders, which curled up at the rounded rows' corners (most visible in dark mode).
- **BREAKING** (API): `PUT /api/preferences` now requires `theme`; a request without it is rejected with 400. The web UI is the only client and sends it.

## Capabilities

### New Capabilities

### Modified Capabilities
- `preferences`: adds the `theme` preference, its default, and its validation on save.
- `web-ui`: adds the header dark mode switch, the dark theme and its first paint, and dark-mode category tints.

## Impact

- Backend: preferences domain (new `Theme` value object), HTTP preferences DTO and handler, SQLite preferences repository (JSON document, no migration).
- Web UI: `index.html` (first-paint script), theme tokens in `index.css`, new `ThemeToggle` and `useTheme`, category tint helpers and their callers, `VideoListPane` separators, a few hard-coded colors (swatch borders, warning icon, dialog backdrop).
- E2E: default preferences fixture gains `theme`; new preferences spec for the saved theme.
