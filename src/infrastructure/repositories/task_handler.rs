pub trait TaskHandler: Send + Sync {
    /// `is_last_attempt` tells the handler whether the task queue has no
    /// retries left for this task, so it can distinguish a transient failure
    /// from a permanent one (e.g. to pick between two failure statuses on
    /// its own aggregate). Handlers that don't need this distinction ignore
    /// it.
    fn handle(&self, payload: &str, is_last_attempt: bool) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeHandler;

    impl TaskHandler for FakeHandler {
        fn handle(&self, _payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn it_should_compile_against_a_fake_handler() {
        let handler: Box<dyn TaskHandler> = Box::new(FakeHandler);

        assert!(handler.handle("{}", false).is_ok());
    }
}
