use crate::messaging::{message::Msg, processor::Executor};
use anyhow::Result;
use async_trait::async_trait;

use super::{Asset, Exchange, MarketOrder, OrderType};

#[derive(Debug, PartialEq)]
pub struct Trader<'a, E>
where
    E: Exchange,
{
    exchange: &'a mut E,
}

impl<'a, E> Trader<'a, E>
where
    E: Exchange,
{
    pub fn new(exchange: &'a mut E) -> Self {
        Trader { exchange }
    }
}

#[async_trait]
impl<'a, E> Executor<'a> for Trader<'a, E>
where
    E: Exchange + Send + Sync,
{
    async fn execute(&mut self, msg: &Msg<'a>) -> Result<()> {
        match msg {
            Msg::Buy => {
                let assets = self.exchange.fetch_assets().await?;
                do_execute(self.exchange, assets.fiat, OrderType::Buy).await
            }
            Msg::Sell => {
                let assets = self.exchange.fetch_assets().await?;
                do_execute(self.exchange, assets.coin, OrderType::Sell).await
            }
            _ => Ok(()),
        }
    }
}

async fn do_execute<'a, E>(
    exchange: &'a mut E,
    asset: Option<Asset>,
    order_type: OrderType,
) -> Result<()>
where
    E: Exchange,
{
    if let Some(asset) = asset {
        if asset.amount > 0.0 {
            let result = exchange
                .place_market_order(&MarketOrder {
                    bid_currency: "EUR".into(),
                    ask_currency: "BTC".into(),
                    amount: asset.amount,
                    order_type,
                })
                .await;
            return result;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::exchange::{Assets, MockExchange};

    use super::*;
    use pretty_assertions::assert_eq;

    #[async_std::test]
    async fn should_buy_max_amount_if_fiat_exists() {
        let mut exchange = MockExchange::new(Assets {
            fiat: Some(Asset {
                amount: 40.0,
                name: "EUR".into(),
            }),
            coin: None,
        });
        let mut trader = Trader::new(&mut exchange);
        trader.execute(&Msg::Buy).await.unwrap();
        let expected = vec![MarketOrder {
            bid_currency: "EUR".into(),
            ask_currency: "BTC".into(),
            amount: 40.0,
            order_type: OrderType::Buy,
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_buy_different_max_amount_if_fiat_exists() {
        let mut exchange = MockExchange::new(Assets {
            fiat: Some(Asset {
                amount: 50.0,
                name: "EUR".into(),
            }),
            coin: None,
        });
        let mut trader = Trader::new(&mut exchange);
        trader.execute(&Msg::Buy).await.unwrap();
        let expected = vec![MarketOrder {
            bid_currency: "EUR".into(),
            ask_currency: "BTC".into(),
            amount: 50.0,
            order_type: OrderType::Buy,
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_not_buy_if_fiat_not_exists() {
        let mut exchange = MockExchange::new(Assets {
            ..Default::default()
        });
        let mut trader = Trader::new(&mut exchange);
        trader.execute(&Msg::Buy).await.unwrap();
        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_not_buy_if_fiat_is_zero() {
        let mut exchange = MockExchange::new(Assets {
            fiat: Some(Asset {
                amount: 0.0,
                name: "EUR".into(),
            }),
            coin: None,
        });
        let mut trader = Trader::new(&mut exchange);
        trader.execute(&Msg::Buy).await.unwrap();
        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_sell_max_amount_if_coin_exists() {
        let mut exchange = MockExchange::new(Assets {
            fiat: None,
            coin: Some(Asset {
                amount: 0.0000001,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);
        trader.execute(&Msg::Sell).await.unwrap();
        let expected = vec![MarketOrder {
            bid_currency: "EUR".into(),
            ask_currency: "BTC".into(),
            amount: 0.0000001,
            order_type: OrderType::Sell,
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_sell_different_max_amount_if_coin_exists() {
        let mut exchange = MockExchange::new(Assets {
            fiat: None,
            coin: Some(Asset {
                amount: 0.0002,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);
        trader.execute(&Msg::Sell).await.unwrap();
        let expected = vec![MarketOrder {
            bid_currency: "EUR".into(),
            ask_currency: "BTC".into(),
            amount: 0.0002,
            order_type: OrderType::Sell,
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_not_sell_if_coin_not_exists() {
        let mut exchange = MockExchange::new(Assets {
            ..Default::default()
        });
        let mut trader = Trader::new(&mut exchange);
        trader.execute(&Msg::Sell).await.unwrap();
        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_not_sell_if_coin_is_zero() {
        let mut exchange = MockExchange::new(Assets {
            fiat: None,
            coin: Some(Asset {
                amount: 0.0,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);
        trader.execute(&Msg::Sell).await.unwrap();
        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }
}
