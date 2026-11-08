use crate::domain::task::Task;

pub trait TaskHandler: Send + Sync {
    /// Runs `task`, which is always of the type this handler is registered
    /// for. `is_last_attempt` tells the handler whether the task queue has no
    /// retries left for this task, so it can distinguish a transient failure
    /// from a permanent one (e.g. to pick between two failure statuses on
    /// its own aggregate). Handlers that don't need this distinction ignore
    /// it.
    fn handle(&self, task: &Task, is_last_attempt: bool) -> anyhow::Result<()>;
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
        fn handle(&self, _task: &Task, _is_last_attempt: bool) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn it_should_compile_against_a_fake_handler() {
        let handler: Box<dyn TaskHandler> = Box::new(FakeHandler);

        assert!(handler.handle(&Task::UpdateYtdlp, false).is_ok());
    }
}
