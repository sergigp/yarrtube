## ADDED Requirements

### Requirement: Header Dark Mode Switch
The header SHALL show, immediately before the settings control, a switch with the accessible name "Dark mode", checked while the UI is drawn in the dark theme. It SHALL be drawn as a pill-shaped track with a round knob: in light mode the knob sits at the start of a neutral track and shows a sun; in dark mode it sits at the end of a track in the primary green and shows a moon. Activating it SHALL draw the UI in the other theme at once and save that theme as the `theme` preference. When the save fails, the UI SHALL be drawn in the saved theme again. The switch SHALL be disabled until the preferences have loaded.

#### Scenario: Turning dark mode on
- **WHEN** a user in light mode activates the "Dark mode" switch
- **THEN** the UI is drawn in the dark theme, the switch is checked and shows a moon, and the `theme` preference is saved as `"dark"` along with every other preference unchanged

#### Scenario: Turning dark mode off
- **WHEN** a user in dark mode activates the "Dark mode" switch
- **THEN** the UI is drawn in the light theme, the switch is unchecked and shows a sun, and the `theme` preference is saved as `"light"`

#### Scenario: Save fails
- **WHEN** a user activates the "Dark mode" switch and saving the preferences fails
- **THEN** the UI returns to the theme it had before and the switch to its previous state

#### Scenario: Preferences still loading
- **WHEN** the preferences have not loaded yet
- **THEN** the "Dark mode" switch is disabled

### Requirement: Saved Theme
The web UI SHALL be drawn in the theme saved as the `theme` preference, on every browser and device, once the preferences load. To avoid a flash of the wrong theme on load, the browser SHALL remember the last theme it drew and draw it before the first paint; when the saved preference differs, the saved preference SHALL win as soon as it loads. In the dark theme, the browser's theme color SHALL match the dark header background.

#### Scenario: Another device chose dark mode
- **WHEN** the `theme` preference is `"dark"` and a user opens the web UI in a browser that has never drawn it
- **THEN** the UI is drawn in the dark theme once the preferences load and the "Dark mode" switch is checked

#### Scenario: Reloading in dark mode
- **WHEN** a user whose `theme` preference is `"dark"` reloads the page
- **THEN** the page is drawn dark from its first paint, without showing the light theme first

#### Scenario: Saved theme overrides the browser's last one
- **WHEN** a browser last drew the dark theme and the `theme` preference has since been saved as `"light"` elsewhere
- **THEN** the UI is drawn in the light theme once the preferences load

### Requirement: Category Tints In Dark Mode
In the dark theme, every place that fills a chip or header with the soft tint of a category's hue (home card chips, the chosen chips of the "Categories" field, a category view's header) SHALL instead use a dark tint of that hue, and every chip name drawn in the dark shade of that hue SHALL instead use a light shade of it. For every palette color, the light shade on the dark tint SHALL have a contrast ratio of at least 4.5:1, and the dark tint SHALL keep the category's hue.

#### Scenario: Home card chip in dark mode
- **WHEN** a user in dark mode views a home card for a video in the "Chess" category whose color is Sky
- **THEN** the "Chess" chip is filled with a dark tint of sky blue and its name is in a light shade of sky blue

#### Scenario: Category header in dark mode
- **WHEN** a user in dark mode opens the "Chess" category view
- **THEN** its header is filled with a dark tint of the category's hue
