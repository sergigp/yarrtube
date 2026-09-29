/// The executor lane a task runs in. Each lane has its own concurrency cap;
/// `Exclusive` runs alone, with nothing else running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskLane {
    Download,
    Thumbnail,
    Light,
    Exclusive,
}
