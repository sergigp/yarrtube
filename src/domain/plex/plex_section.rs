use std::path::Path;

/// A Plex library section and the server-side folders it scans, so a
/// folder can be routed to the sections that contain it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexSection {
    pub id: String,
    pub locations: Vec<String>,
}

impl PlexSection {
    /// Whether `path` (server-side) lies within one of the section's
    /// locations, comparing whole path segments.
    pub fn contains(&self, path: &str) -> bool {
        self.locations
            .iter()
            .any(|location| Path::new(path).starts_with(location))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_contain_a_path_under_one_of_its_locations() {
        assert!(section().contains("/volume1/media/yarrtube/channels/Some video"));
    }

    #[test]
    fn it_should_contain_a_path_under_a_location_with_a_trailing_slash() {
        let section = PlexSection {
            locations: vec!["/volume1/media/yarrtube/playlists/".to_string()],
            ..section()
        };

        assert!(section.contains("/volume1/media/yarrtube/playlists/kids/Some video"));
    }

    #[test]
    fn it_should_not_contain_a_path_outside_its_locations() {
        assert!(!section().contains("/volume1/media/yarrtube/channels-old/Some video"));
    }

    #[test]
    fn it_should_not_contain_anything_without_locations() {
        let section = PlexSection {
            locations: vec![],
            ..section()
        };

        assert!(!section.contains("/volume1/media/yarrtube/playlists/kids/Some video"));
    }

    fn section() -> PlexSection {
        PlexSection {
            id: "19".to_string(),
            locations: vec![
                "/volume1/media/yarrtube/playlists".to_string(),
                "/volume1/media/yarrtube/channels".to_string(),
            ],
        }
    }
}
