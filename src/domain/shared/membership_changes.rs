/// What one reconcile pass's membership sync with YouTube did, for its
/// summary line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MembershipChanges {
    /// How many videos YouTube listed; `None` when the listing failed and
    /// membership was left untouched.
    pub listed: Option<usize>,
    pub added: usize,
    pub removed: usize,
    pub listing_seconds: f64,
}
