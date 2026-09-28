/// How many videos each home section holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeLimits {
    pub continue_watching: usize,
    pub quick_watches: usize,
    pub latest: usize,
}
