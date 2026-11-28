pub trait EventSubscriber: Send + Sync {
    fn handle(&self, payload: &str) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeSubscriber;

    impl EventSubscriber for FakeSubscriber {
        fn handle(&self, _payload: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn it_should_compile_against_a_fake_subscriber() {
        let subscriber: Box<dyn EventSubscriber> = Box::new(FakeSubscriber);

        assert!(subscriber.handle("{}").is_ok());
    }
}
