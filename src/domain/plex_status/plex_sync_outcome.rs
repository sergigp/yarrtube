use crate::domain::plex::{PlexIdentity, PlexSection};

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

impl PlexSyncOutcome {
    /// It reached the server `identity` and found yarrtube's folders in
    /// `sections`.
    pub fn reached(identity: &PlexIdentity, sections: &[PlexSection]) -> Self {
        Self::Reached {
            version: identity.version.clone(),
            libraries: sections
                .iter()
                .map(|section| section.title.clone())
                .collect(),
        }
    }
}
