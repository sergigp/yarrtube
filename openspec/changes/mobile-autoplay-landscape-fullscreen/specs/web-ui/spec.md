## ADDED Requirements

### Requirement: Chosen Video Starts Playing
On every viewport, a downloaded video the user chooses SHALL start playing without a tap on play: one opened from a card or link (deep-linked) or picked in the watching queue. The video shown by default, a refresh of the list, and a selected video that finishes downloading while selected SHALL NOT start it. When the browser refuses to start playback, the video SHALL stay paused, ready to play from its controls, with no error shown.

#### Scenario: Opening a card on a phone
- **WHEN** a user on a phone taps a downloaded video's card on the home view
- **THEN** the watching layout opens with that video playing

#### Scenario: Picking from the queue
- **WHEN** a user, on a phone or a desktop, picks another downloaded video in the watching queue
- **THEN** the player switches to that video and it plays, in the same player

#### Scenario: Default video
- **WHEN** a user opens a channel's view without a video in the address
- **THEN** its first video is shown paused

#### Scenario: List refresh
- **WHEN** the user has paused the chosen video and the video list refreshes
- **THEN** the video stays paused

#### Scenario: Download finishes while selected
- **WHEN** a user picks a video that is still downloading, and its download finishes while it is selected
- **THEN** the player shows it paused

#### Scenario: Browser refuses playback
- **WHEN** a user picks a video and the browser refuses to start playback
- **THEN** the video is shown paused with its controls, and no error is shown

### Requirement: Landscape Player Takeover On Phones
On a phone (touch screen whose height in landscape is at most 32rem) held in landscape, while the player holds a downloaded video, playing or paused, only the video SHALL show: the player SHALL fill the viewport on black, the video letterboxed and kept clear of the safe-area insets, its native controls usable, with the chapter rail and the page hidden and the page not scrollable. Rotating back to portrait SHALL restore the page at its previous scroll position, without interrupting playback.

#### Scenario: Rotating to landscape while playing
- **WHEN** a user on a phone rotates to landscape while a downloaded video plays in the watching layout
- **THEN** the video fills the screen on black, letterboxed, keeps playing, and no header, chapter rail or page content shows

#### Scenario: Rotating to landscape while paused
- **WHEN** a user on a phone rotates to landscape while a downloaded video is paused in the player
- **THEN** the video fills the screen, still paused, with its controls usable

#### Scenario: Rotating back to portrait
- **WHEN** a user scrolled 400 px down the watching layout, rotates to landscape and back to portrait while the video plays
- **THEN** the page shows again scrolled 400 px down, and the video plays on without restarting or pausing

#### Scenario: Video not downloaded
- **WHEN** a user on a phone rotates to landscape while the selected video is not downloaded, or no video is selected
- **THEN** the page shows as usual, with no takeover

#### Scenario: Notch
- **WHEN** a user on a phone with a notch rotates to landscape with a downloaded video in the player
- **THEN** the video and its controls stay clear of the notch, the area beside it black

#### Scenario: Desktop or tablet
- **WHEN** a user with a mouse, or on a tablet taller than 32rem in landscape, uses a landscape window
- **THEN** the page shows as usual, with no takeover
