## MODIFIED Requirements

### Requirement: Transcript Stored
A successful subtitle fetch SHALL store the video's transcript: the captions' plain text, without cue numbers, timings or formatting tags and with lines repeated by rolling auto-generated captions collapsed, together with its language and whether it came from uploaded (manual) or auto-generated captions. Only a cue's number line, its timing line and markup tags (e.g. `<i>`, `</i>`, `<font …>`, inline `<00:01:02.345>` timestamps) SHALL be removed, and subtitle escape codes SHALL be replaced by what they stand for: `\h` (hard space) by a space and `\N` (forced line break) by a line break, with runs of spaces collapsed into one. Every other caption text SHALL be kept, including lines made only of digits, a literal `<` and text containing `-->`. A video SHALL have at most one transcript, and it SHALL be deleted whenever the video's record is deleted, including when its whole playlist or channel is deleted.

#### Scenario: Auto-generated rolling captions
- **WHEN** a fetched auto-generated track repeats each line across consecutive cues as it scrolls
- **THEN** the stored transcript contains each spoken line once, in order

#### Scenario: Video removed
- **WHEN** a video with a stored transcript is removed from its playlist or channel and its record deleted
- **THEN** its transcript is deleted too

#### Scenario: Channel deleted
- **WHEN** a tracked channel whose videos have stored transcripts is deleted
- **THEN** those transcripts are deleted too

#### Scenario: Caption text that looks like markup
- **WHEN** a fetched track has cues whose text is `2024`, `if x < 5` and `use x --> y`
- **THEN** the stored transcript contains those three lines unchanged

#### Scenario: Hard spaces
- **WHEN** a fetched track has a cue whose lines are `play the Jabava\h` and `London\h\h`
- **THEN** the stored transcript contains the lines `play the Jabava` and `London`

#### Scenario: Forced line break
- **WHEN** a fetched track has a cue whose text is `first line\Nsecond\h\hline`
- **THEN** the stored transcript contains the lines `first line` and `second line`
