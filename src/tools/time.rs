use std::time::{SystemTime, UNIX_EPOCH};

pub type Timestamp = u128;

pub trait TimeProvider {
    fn now(&mut self) -> Timestamp;
}

pub struct TimeProviderImpl {}

impl TimeProviderImpl {
    pub fn new() -> Self {
        TimeProviderImpl {}
    }
}

impl TimeProvider for TimeProviderImpl {
    fn now(&mut self) -> Timestamp {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_micros()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    pub struct MockTimeProvider {
        counter: u128,
    }

    impl MockTimeProvider {
        pub fn new() -> Self {
            MockTimeProvider { counter: 0 }
        }
    }

    impl TimeProvider for MockTimeProvider {
        fn now(&mut self) -> Timestamp {
            let now = self.counter;
            self.counter += 1;
            now
        }
    }

    //TODO: Check if this really executes the right way (just switching buy and sell and keeping currencies the same)
    #[test]
    fn mock_now_returns_time() {
        let mut time_provider = MockTimeProvider::new();
        assert_eq!(0, time_provider.now());
    }

    #[test]
    fn mock_now_returns_same_time() {
        let mut time_provider = MockTimeProvider::new();
        assert_eq!(0, time_provider.now());
    }

    #[test]
    fn mock_now_returns_different_time() {
        let mut time_provider = MockTimeProvider::new();
        time_provider.now();
        assert_eq!(1, time_provider.now());
    }
}
