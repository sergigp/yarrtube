# Spec Delta

## MODIFIED Requirements

### Requirement: Smart Playlist Of Every Category Episode
For each category with at least one labelled episode in a Plex library, the system SHALL keep one smart playlist in that library, owned by the token's Plex user, listing the episodes of that library labelled with the category: watched or not while `plex_playlist_watched` is `"keep"`, only unwatched ones while it is `"drop"`; by air date and, within one air date, episode number, both descending while `plex_playlist_order` is `"newest_first"` and both ascending while it is `"oldest_first"`.

#### Scenario: Category with episodes in one library
- **WHEN** episodes labelled "Chess" exist only in the library "Subscriptions", with the default preferences
- **THEN** one smart playlist titled "Chess" exists for "Subscriptions", listing all its "Chess" episodes newest first

#### Scenario: Watched episode stays in the playlist
- **WHEN** `plex_playlist_watched` is `"keep"` and an episode of the "Chess" playlist is watched in Plex or in the web UI
- **THEN** Plex still lists it in the playlist without any action by yarrtube

#### Scenario: Watched episode drops out
- **WHEN** `plex_playlist_watched` is `"drop"` and an episode of the "Chess" playlist is watched in Plex or in the web UI
- **THEN** Plex no longer lists it in the playlist, without any action by yarrtube

#### Scenario: Same-day episodes ordered by episode number
- **WHEN** a channel in "Chess" has the episodes `S2026E031501` and `S2026E031502`, both aired on 15 March 2026, with the default preferences
- **THEN** the playlist lists `S2026E031502` before `S2026E031501`

#### Scenario: Oldest first
- **WHEN** `plex_playlist_order` is `"oldest_first"` and "Chess" has episodes aired on 1 and 15 March 2026
- **THEN** the playlist lists the 1 March episode first

#### Scenario: Category spanning libraries
- **WHEN** episodes labelled "Chess" exist in the libraries "Subscriptions" and "Sergi's Playlists"
- **THEN** one playlist exists for each, titled "Chess · Subscriptions" and "Chess · Sergi's Playlists"

## ADDED Requirements

### Requirement: Playlists Follow The Playlist Preferences
During each label sync, the system SHALL replace every category playlist it created whose listing differs from the current `plex_playlist_watched` and `plex_playlist_order`, or whose listing it doesn't know (created before it recorded one). A playlist matching them SHALL be kept.

#### Scenario: Preference changed
- **WHEN** the "Chess" playlist keeps watched episodes and a user saves `plex_playlist_watched` `"drop"`
- **THEN** after the label sync that follows, "Chess" lists only unwatched episodes, under the same title

#### Scenario: Preferences unchanged
- **WHEN** a label sync runs and every playlist matches the preferences
- **THEN** no playlist is deleted or created

#### Scenario: Playlist from an older version
- **WHEN** the daemon starts after this change with a "Chess" playlist created by an earlier version
- **THEN** the first label sync replaces it with one that follows the preferences, and later syncs keep it

#### Scenario: Replacement fails
- **WHEN** Plex rejects creating the replacement for "Chess"
- **THEN** a later label sync creates it
