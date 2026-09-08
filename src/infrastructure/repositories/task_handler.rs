pub trait TaskHandler: Send + Sync {
    fn handle(&self, payload: &str) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeHandler;

    impl TaskHandler for FakeHandler {
        fn handle(&self, _payload: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn it_should_compile_against_a_fake_handler() {
        let handler: Box<dyn TaskHandler> = Box::new(FakeHandler);

        assert!(handler.handle("{}").is_ok());
    }
}
