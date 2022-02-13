use uuid;

pub type Uuid = uuid::Uuid;

pub trait IdProvider {
    fn new_random(&mut self) -> Uuid;
}

#[derive(Clone)]
pub struct UuidProvider {}

impl UuidProvider {
    pub fn new() -> Self {
        UuidProvider {}
    }
}

impl IdProvider for UuidProvider {
    fn new_random(&mut self) -> Uuid {
        uuid::Uuid::new_v4()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Clone)]
    pub struct MockUuidProvider {
        counter: u128,
    }

    impl MockUuidProvider {
        pub fn new() -> Self {
            MockUuidProvider { counter: 0 }
        }
    }

    impl IdProvider for MockUuidProvider {
        fn new_random(&mut self) -> Uuid {
            let uuid = uuid::Uuid::from_u128(self.counter);
            self.counter += 1;
            uuid
        }
    }

    #[test]
    fn mock_new_random_returns_uuid() {
        let mut uuid_provider = MockUuidProvider::new();
        assert_eq!(
            "00000000-0000-0000-0000-000000000000",
            uuid_provider.new_random().to_string()
        );
    }

    #[test]
    fn mock_new_random_returns_same_uuid() {
        let mut uuid_provider = MockUuidProvider::new();
        assert_eq!(
            "00000000-0000-0000-0000-000000000000",
            uuid_provider.new_random().to_string()
        );
    }

    #[test]
    fn mock_new_random_returns_different_second_uuid() {
        let mut uuid_provider = MockUuidProvider::new();
        uuid_provider.new_random();
        assert_eq!(
            "00000000-0000-0000-0000-000000000001",
            uuid_provider.new_random().to_string()
        );
    }
}
