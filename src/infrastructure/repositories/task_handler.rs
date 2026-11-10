use crate::domain::task::{Task, TaskOutcome};

pub trait TaskHandler: Send + Sync {
    /// Runs `task`, which is always of the type this handler is registered
    /// for: done, or to run again later (see the `task-scheduling`
    /// capability). An error is for failures the domain doesn't handle
    /// itself (e.g. the database being unavailable): the task queue retries
    /// and then dead-letters the task.
    fn handle(&self, task: &Task) -> anyhow::Result<TaskOutcome>;
}

/// The error a handler returns when handed a task of a type it isn't
/// registered for, which only a miswired registry can cause.
pub fn unexpected_task(task: &Task) -> anyhow::Error {
    anyhow::anyhow!("unexpected {} task for this handler", task.task_type())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeHandler;

    impl TaskHandler for FakeHandler {
        fn handle(&self, _task: &Task) -> anyhow::Result<TaskOutcome> {
            Ok(TaskOutcome::Done)
        }
    }

    #[test]
    fn it_should_compile_against_a_fake_handler() {
        let handler: Box<dyn TaskHandler> = Box::new(FakeHandler);

        assert!(handler.handle(&Task::UpdateYtdlp).is_ok());
    }
}
