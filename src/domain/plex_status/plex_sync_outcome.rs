/// How a Plex sync ended: what it saw of Plex, or why it failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlexSyncOutcome {
    /// It reached Plex `version`, whose `libraries` hold yarrtube's folders.
    Reached {
        version: String,
        libraries: Vec<String>,
    },
    Failed {
        error: String,
    },
}
