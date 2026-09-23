## MODIFIED Requirements

### Requirement: Add Dialog Download Options
The add dialog SHALL present video quality and, in channel mode, the video limit, inside a collapsed "Advanced options" section that is not expanded by default. The video quality control SHALL be labeled "Video quality" and SHALL offer a tooltip explaining that it controls the download resolution and that a lower resolution reduces storage use. In channel mode, the video limit field SHALL default to 3 and SHALL accept whole numbers from 1 to 1000.

The storage location is not part of this section; it is presented in the dialog's main body.

#### Scenario: Advanced options start collapsed
- **WHEN** a user opens the add dialog in either mode
- **THEN** the video quality control and (in channel mode) the video limit field are hidden inside a collapsed "Advanced options" section

#### Scenario: Video quality tooltip
- **WHEN** a user reveals the "Video quality" tooltip
- **THEN** it explains that the setting controls the download resolution and that choosing a lower resolution saves storage

#### Scenario: Channel video limit default
- **WHEN** a user opens the add dialog in channel mode and does not change the video limit
- **THEN** the video limit field defaults to 3

#### Scenario: Channel video limit range
- **WHEN** a user enters a video limit below 1 or above 1000 in channel mode
- **THEN** the dialog flags the field as invalid and does not submit the request
