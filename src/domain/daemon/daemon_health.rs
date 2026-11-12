use std::collections::BTreeMap;

/// How many videos are pending, retrying and failed for one step.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepCounts {
    pub pending: usize,
    pub retrying: usize,
    pub failed: usize,
}

/// An AI step: its tasks due to run, waiting for the AI, and the videos
/// retrying or failed for it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiStepCounts {
    pub queued: usize,
    pub retrying: usize,
    pub failed: usize,
}

/// What the daemon's heartbeat reports — see the `daemon` capability: the
/// task and event queues, their dead letters, and the library's videos by
/// step status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonHealth {
    /// Pending tasks per lane name; lanes with none are left out.
    pub pending_tasks: BTreeMap<&'static str, usize>,
    pub running_tasks: usize,
    /// How long the oldest pending task due to run has waited past its run
    /// time; `None` when no pending task is due.
    pub oldest_eligible_task_age_seconds: Option<i64>,
    pub pending_events: usize,
    pub dead_lettered_tasks: usize,
    pub dead_lettered_events: usize,
    pub downloads: StepCounts,
    pub summaries: AiStepCounts,
    pub categorizations: AiStepCounts,
}
