/// What one check of a piece of late YouTube data found — see the
/// `late-video-data` capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LateDataOutcome {
    /// The data was found, or reached its target.
    Complete,
    /// A conclusive check found the data missing, or below its target.
    Incomplete,
    /// The downloader failed, or the check was inconclusive.
    Failed,
}
