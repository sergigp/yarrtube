## ADDED Requirements

### Requirement: Home And Category Videos Include Their Categories
Each video returned by the home endpoint and the category videos endpoint SHALL include the identifiers of the categories it belongs to, ordered by category name ignoring case, each once. A video SHALL belong to a category when any of the category's members has that YouTube video downloaded, whichever source the video is returned from. Members whose unwatched count is hidden or that are excluded from home SHALL still count. A video in no category SHALL include an empty list.

#### Scenario: Video in one category
- **WHEN** a home video comes from the channel "@Hikaru", a member of "Chess" only
- **THEN** its category identifiers are those of ["Chess"]

#### Scenario: Ordered by category name
- **WHEN** the source of a home video is a member of "Streams" and "chess"
- **THEN** its category identifiers are those of ["chess", "Streams"]

#### Scenario: Category reached through another source
- **WHEN** a YouTube video is returned from the channel "@Hikaru", a member of "Chess", and is also tracked by the playlist "PLbest", a member of "Highlights" and excluded from home
- **THEN** its category identifiers are those of ["Chess", "Highlights"]

#### Scenario: Category reached by two members once
- **WHEN** a YouTube video is tracked by two members of "Chess"
- **THEN** "Chess" appears once among its category identifiers

#### Scenario: No category
- **WHEN** a home video's YouTube video is tracked by no member of any category
- **THEN** its category identifiers are an empty list

#### Scenario: Category videos include their categories
- **WHEN** a video listed for the category "Chess" is also tracked by a member of "Streams"
- **THEN** its category identifiers are those of ["Chess", "Streams"]
