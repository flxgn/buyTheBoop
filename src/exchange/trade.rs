use crate::messaging::{message, message::Msg, message::MsgData, processor::Actor};
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
impl<'a, E> Actor for Trader<'a, E>
where
    E: Exchange + Send + Sync,
{
    async fn act(&mut self, msg: &Msg) -> Result<Vec<MsgData>> {
        let res = match msg.data {
            MsgData::Buy => {
                let assets = self.exchange.fetch_assets().await?;
                execute(self.exchange, assets.quote, OrderType::Buy).await?
            }
            MsgData::Sell => {
                let assets = self.exchange.fetch_assets().await?;
                execute(self.exchange, assets.base, OrderType::Sell).await?
            }
            _ => vec![],
        };
        Ok(res)
    }
}

async fn execute<'a, E>(
    exchange: &'a mut E,
    asset: Option<Asset>,
    order_type: OrderType,
) -> Result<Vec<MsgData>>
where
    E: Exchange,
{
    if let Some(asset) = asset {
        if asset.amount > 0.0 {
            let order = MarketOrder {
                base: "BTC".into(),
                quote: "USDT".into(),
                amount: asset.amount,
                order_type,
            };
            return exchange.place_market_order(&order).await.map(|_| {
                vec![MsgData::OrderExecuted(message::MarketOrder {
                    amount: order.amount,
                    quote: order.quote,
                    base: order.base,
                    order_type: match order.order_type {
                        OrderType::Buy => message::OrderType::Buy,
                        OrderType::Sell => message::OrderType::Sell,
                    },
                })]
            });
        }
    }
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use crate::exchange::{Assets, MockExchange};

    use super::*;
    use pretty_assertions::assert_eq;

    //TODO: Check if this really executes the right way (just switching buy and sell and keeping currencies the same)
    #[async_std::test]
    async fn should_buy_max_amount_if_fiat_exists() {
        let mut exchange = MockExchange::new(Assets {
            quote: Some(Asset {
                amount: 40.0,
                name: "USDT".into(),
            }),
            base: None,
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Buy)).await.unwrap();

        let expected = vec![MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 40.0,
            order_type: OrderType::Buy,
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_buy_different_max_amount_if_fiat_exists() {
        let mut exchange = MockExchange::new(Assets {
            quote: Some(Asset {
                amount: 50.0,
                name: "USDT".into(),
            }),
            base: None,
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Buy)).await.unwrap();

        let expected = vec![MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
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

        trader.act(&Msg::with_data(MsgData::Buy)).await.unwrap();

        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_not_buy_if_fiat_is_zero() {
        let mut exchange = MockExchange::new(Assets {
            quote: Some(Asset {
                amount: 0.0,
                name: "USDT".into(),
            }),
            base: None,
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Buy)).await.unwrap();

        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_create_buy_order_executed_event() {
        let mut exchange = MockExchange::new(Assets {
            quote: Some(Asset {
                amount: 50.0,
                name: "USDT".into(),
            }),
            base: None,
        });
        let mut trader = Trader::new(&mut exchange);

        let actual = trader.act(&Msg::with_data(MsgData::Buy)).await.unwrap();

        let expected = vec![MsgData::OrderExecuted(message::MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 50.0,
            order_type: message::OrderType::Buy,
        })];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_sell_max_amount_if_coin_exists() {
        let mut exchange = MockExchange::new(Assets {
            quote: None,
            base: Some(Asset {
                amount: 0.0000001,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Sell)).await.unwrap();

        let expected = vec![MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 0.0000001,
            order_type: OrderType::Sell,
        }];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_sell_different_max_amount_if_coin_exists() {
        let mut exchange = MockExchange::new(Assets {
            quote: None,
            base: Some(Asset {
                amount: 0.0002,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Sell)).await.unwrap();

        let expected = vec![MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
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

        trader.act(&Msg::with_data(MsgData::Sell)).await.unwrap();

        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_not_sell_if_coin_is_zero() {
        let mut exchange = MockExchange::new(Assets {
            quote: None,
            base: Some(Asset {
                amount: 0.0,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);

        trader.act(&Msg::with_data(MsgData::Sell)).await.unwrap();

        let expected: Vec<MarketOrder> = vec![];
        let actual = exchange.recorded_orders;
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_create_sell_order_executed_event() {
        let mut exchange = MockExchange::new(Assets {
            quote: None,
            base: Some(Asset {
                amount: 0.0000001,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);

        let actual = trader.act(&Msg::with_data(MsgData::Sell)).await.unwrap();

        let expected = vec![MsgData::OrderExecuted(message::MarketOrder {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 0.0000001,
            order_type: message::OrderType::Sell,
        })];
        assert_eq!(expected, actual)
    }
}
