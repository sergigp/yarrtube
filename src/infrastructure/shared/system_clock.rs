use chrono::{DateTime, Utc};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[cfg(test)]
pub struct FixedClock(pub DateTime<Utc>);

#[cfg(test)]
impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_return_the_configured_value() {
        let fixed = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let clock = FixedClock(fixed);

        assert_eq!(clock.now(), fixed);
    }
}
