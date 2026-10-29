use std::path::Path;

/// A Plex library section and the server-side folders it scans, so a
/// folder can be routed to the sections that contain it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexSection {
    pub id: String,
    pub title: String,
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

    /// Whether a location is under `root` or `root` is under a location.
    pub fn serves(&self, root: &str) -> bool {
        self.locations.iter().any(|location| {
            Path::new(location).starts_with(root) || Path::new(root).starts_with(location)
        })
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

    #[test]
    fn it_should_serve_a_root_under_or_above_its_location() {
        let served: Vec<bool> = [
            "/volume1/media/yarrtube",
            "/volume1/media/yarrtube/channels/",
            "/volume1/media/yarrtube/channels/Some Channel",
            "/volume1/media/yarrtube-old",
            "/volume2",
        ]
        .into_iter()
        .map(|root| section().serves(root))
        .collect();

        assert_eq!(served, vec![true, true, true, false, false]);
    }

    fn section() -> PlexSection {
        PlexSection {
            id: "19".to_string(),
            title: "Subscriptions".to_string(),
            locations: vec![
                "/volume1/media/yarrtube/playlists".to_string(),
                "/volume1/media/yarrtube/channels".to_string(),
            ],
        }
    }
}
