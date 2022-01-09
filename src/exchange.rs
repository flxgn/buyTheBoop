pub mod okex;

use std::iter::Iterator;
use uuid::Uuid;
use async_trait::async_trait;

#[async_trait]
pub trait Exchange {
    async fn event_stream<'a>(&'a self) -> Box<dyn Iterator<Item=ExchangeStreamEvent> + 'a>;

    async fn place_market_order(&self, order: &mut MarketOrder) -> Result<(), ()>;
}

#[derive(Debug, Clone)]
pub enum ExchangeStreamEvent {
    Subscription(Subscription),
    Pair(Pair),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Default)]
pub struct Subscription {
    pub id: Uuid,
    pub bid_currency: String,
    pub ask_currency: String,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Pair {
    pub id: Uuid,
    pub bid_orders: Vec<Order>,
    pub ask_orders: Vec<Order>,
}

#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct Order {
    pub price: f64,
    pub amount: f64,
}

#[derive(Debug, Copy, Clone)]
pub enum OrderType {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct MarketOrder {
    pub bid_currency: String,
    pub ask_currency: String,
    pub order_type: OrderType,
    pub amount: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    struct TestExchange {
        test_event_stream: Vec<ExchangeStreamEvent>
    }

    #[async_trait]
    impl Exchange for TestExchange {
        async fn event_stream<'a>(&'a self) -> Box<dyn Iterator<Item=ExchangeStreamEvent> + 'a> {
            let event_stream = self.test_event_stream.iter().cloned();
            Box::new(event_stream)
        }

        async fn place_market_order(&self, _order: &mut MarketOrder) -> Result<(), ()> {
            Result::Ok(())
        }
    }

    async fn consume_event_stream<T>(exchange: &T)
        where T: Exchange
    {
        for event in exchange.event_stream().await {
            match event {
                ExchangeStreamEvent::Subscription(s) => println!("{:#?}", s),
                ExchangeStreamEvent::Pair(p) => println!("{:#?}", p),
            }
        }
    }

    #[test]
    fn test_consume_event_stream() {
        let s1 = Subscription {
            id: Uuid::new_v3(&Uuid::NAMESPACE_OID, "1".as_bytes()),
            bid_currency: String::from("ETH"),
            ask_currency: String::from("EUR"),
        };
        let s2 = Subscription {
            id: Uuid::new_v3(&Uuid::NAMESPACE_OID, "2".as_bytes()),
            bid_currency: String::from("ETH"),
            ask_currency: String::from("BTC"),
        };
        let s3 = Subscription {
            id: Uuid::new_v3(&Uuid::NAMESPACE_OID, "3".as_bytes()),
            bid_currency: String::from("BTC"),
            ask_currency: String::from("EUR"),
        };
        let event_1 = ExchangeStreamEvent::Subscription(s1);
        let event_2 = ExchangeStreamEvent::Subscription(s2);
        let event_3 = ExchangeStreamEvent::Subscription(s3);
        let test_exchange = TestExchange { test_event_stream: vec![event_1, event_2, event_3] };
        consume_event_stream(&test_exchange);
        assert_eq!(1, 1)
    }
}
