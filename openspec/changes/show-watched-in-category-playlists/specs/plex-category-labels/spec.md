## REMOVED Requirements

### Requirement: Smart Playlist Per Category And Library
**Reason**: Listed only unwatched episodes, so a watched video left the Plex playlist while staying on the web UI's category page; its "Watched episode leaves the playlist" scenario no longer holds. Replaced by "Smart Playlist Of Every Category Episode".
**Migration**: See this capability's "Smart Playlist Of Every Category Episode" requirement. Playlists created before keep the old filter until deleted in Plex; the next label sync recreates them with the new one.

## ADDED Requirements

### Requirement: Smart Playlist Of Every Category Episode
For each category with at least one labelled episode in a Plex library, the system SHALL keep one smart playlist in that library, owned by the token's Plex user, listing every episode of that library labelled with the category, watched or not, newest air date first and, within one air date, highest episode number first.

#### Scenario: Category with episodes in one library
- **WHEN** episodes labelled "Chess" exist only in the library "Subscriptions"
- **THEN** one smart playlist titled "Chess" exists for "Subscriptions", listing all its "Chess" episodes newest first

#### Scenario: Watched episode stays in the playlist
- **WHEN** an episode of the "Chess" playlist is watched in Plex or in the web UI
- **THEN** Plex still lists it in the playlist, shown as watched, without any action by yarrtube

#### Scenario: Same-day episodes ordered by episode number
- **WHEN** a channel in "Chess" has the episodes `S2026E031501` and `S2026E031502`, both aired on 15 March 2026
- **THEN** the playlist lists `S2026E031502` before `S2026E031501`

#### Scenario: Category spanning libraries
- **WHEN** episodes labelled "Chess" exist in the libraries "Subscriptions" and "Sergi's Playlists"
- **THEN** one playlist exists for each, titled "Chess · Subscriptions" and "Chess · Sergi's Playlists"
