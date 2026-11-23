/// Why YouTube won't serve a video. Recheckable ones may become public later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unavailability {
    MembersOnly,
    Private,
    GeoBlocked,
    /// "claimed content" and "copyright".
    Copyright,
    /// "has been removed", "no longer available".
    Removed,
    AccountTerminated,
}

impl Unavailability {
    /// MembersOnly and Private.
    pub fn is_recheckable(self) -> bool {
        false
    }
}
