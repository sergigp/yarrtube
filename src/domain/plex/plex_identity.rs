/// Who the Plex server is and whose watch state the token reads and writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexIdentity {
    /// The server's machine identifier.
    pub server_id: String,
    /// The Plex Media Server version, e.g. `1.43.5.11079-b3b13e506`.
    pub version: String,
    /// `"<machineIdentifier>/<token fingerprint>"`.
    pub viewer_id: String,
}
