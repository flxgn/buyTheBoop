pub mod okex;
pub mod simulation;
pub mod trade;

use crate::messaging::message::{MessageId, Msg};
use anyhow::Result;
use async_trait::async_trait;
use hmac::digest::generic_array::typenum::Or;
use std::iter::Iterator;
use uuid::Uuid;

pub type Amount = f64;

#[async_trait]
pub trait Exchange {
    async fn event_stream(&self) -> Box<dyn Iterator<Item = Msg>>;

    async fn place_market_order(&mut self, order: &MarketOrder) -> Result<Amount>;

    async fn fetch_assets(&self) -> Result<Assets>;
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct ExchangeOptions {
    pub fee: f64,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Asset {
    pub name: String,
    pub amount: f64,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Assets {
    pub base: Option<Asset>,
    pub quote: Option<Asset>,
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

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum OrderType {
    Buy,
    Sell,
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::Buy
    }
}

#[derive(PartialEq, Debug, Clone, Default)]
pub struct MarketOrder {
    pub correlation_id: MessageId,
    pub base: String,
    pub quote: String,
    pub order_type: OrderType,
    pub amount: f64,
}

#[derive(Default)]
pub struct MockExchange {
    assets: Assets,
    pub recorded_orders: Vec<MarketOrder>,
}

impl MockExchange {
    pub fn new(assets: Assets) -> Self {
        MockExchange {
            assets,
            ..Default::default()
        }
    }
}

#[async_trait]
impl Exchange for MockExchange {
    async fn event_stream(&self) -> Box<dyn Iterator<Item = Msg>> {
        unimplemented!()
    }

    async fn place_market_order(&mut self, order: &MarketOrder) -> Result<Amount> {
        self.recorded_orders.push(order.clone());
        Ok(order.amount)
    }

    async fn fetch_assets(&self) -> Result<Assets> {
        Ok(self.assets.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[async_std::test]
    async fn mock_should_fetch_provided_assets() {
        let given_assets = Assets {
            quote: Some(Asset {
                name: "USD".into(),
                amount: 50.0,
            }),
            base: None,
        };
        let exchange = MockExchange::new(given_assets.clone());
        let actual = exchange.fetch_assets().await.unwrap();
        assert_eq!(given_assets, actual)
    }

    #[async_std::test]
    async fn mock_should_fetch_different_assets() {
        let given_assets = Assets {
            quote: None,
            base: Some(Asset {
                name: "BTW".into(),
                amount: 0.01,
            }),
        };
        let exchange = MockExchange::new(given_assets.clone());
        let actual = exchange.fetch_assets().await.unwrap();
        assert_eq!(given_assets, actual)
    }

    #[async_std::test]
    async fn mock_should_record_placed_marked_orders() {
        let mut exchange = MockExchange::new(Assets {
            ..Default::default()
        });
        let expected_order = MarketOrder {
            base: "EUR".into(),
            quote: "BTC".into(),
            amount: 50.0,
            order_type: OrderType::Buy,
            ..Default::default()
        };
        exchange.place_market_order(&expected_order).await.unwrap();
        assert_eq!(vec![expected_order], exchange.recorded_orders)
    }

    #[async_std::test]
    async fn mock_should_record_different_placed_marked_orders() {
        let mut exchange = MockExchange::new(Assets {
            ..Default::default()
        });
        let expected_order = MarketOrder {
            base: "BTC".into(),
            quote: "EUR".into(),
            amount: 40.0,
            order_type: OrderType::Sell,
            ..Default::default()
        };
        exchange.place_market_order(&expected_order).await.unwrap();
        assert_eq!(vec![expected_order], exchange.recorded_orders)
    }
}
