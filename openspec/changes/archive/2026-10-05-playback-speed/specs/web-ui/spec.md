## ADDED Requirements

### Requirement: Playback Speed Control
The title row of a playlist or channel detail view's video detail pane SHALL show a playback speed control next to the "⋮" actions menu, labelled with the current playback speed (e.g. "1x", "1.5x"). Activating it SHALL open a menu offering the speeds 1x, 1.1x, 1.25x, 1.5x and 2x, with the current speed marked. Choosing a speed SHALL change the playing video's speed immediately without interrupting playback. The speed SHALL apply only to the selected video: selecting another video, or reloading the application, SHALL start playback at 1x. When the speed is changed through the browser's own player controls, the control SHALL show that speed, even when it is not one of the offered speeds. The control SHALL be disabled while the selected video has not finished downloading.

#### Scenario: Choosing a faster speed
- **WHEN** a user opens the speed control of a downloaded video and chooses "1.5x"
- **THEN** the video plays at 1.5 times normal speed and the control reads "1.5x"

#### Scenario: Default speed
- **WHEN** a user selects a downloaded video
- **THEN** the speed control reads "1x" and the video plays at normal speed

#### Scenario: Speed resets when another video is selected
- **WHEN** a user plays a video at 2x and then selects another video
- **THEN** the newly selected video plays at 1x and the control reads "1x"

#### Scenario: Speed resets on reload
- **WHEN** a user plays a video at 1.25x and reloads the page
- **THEN** the video plays at 1x and the control reads "1x"

#### Scenario: Speed changed from the browser's own controls
- **WHEN** a user changes the speed to 1.75x through the browser's native player menu
- **THEN** the speed control reads "1.75x" and none of the offered speeds is marked as current

#### Scenario: Video not downloaded yet
- **WHEN** the selected video is pending, downloading or errored
- **THEN** the speed control is shown disabled
