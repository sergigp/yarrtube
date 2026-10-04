/// A Plex library section and the server-side folders it scans, so a
/// folder can be routed to the sections that contain it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexSection {
    pub id: String,
    pub locations: Vec<String>,
}
