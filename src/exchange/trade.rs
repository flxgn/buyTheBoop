use crate::messaging::{message::Order, message::Msg, message::MsgData, processor::Actor};
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
                match order.order_type {
                    OrderType::Buy => vec![MsgData::Bought(Order {
                        amount: order.amount,
                        quote: order.quote,
                        base: order.base,
                    })],
                    OrderType::Sell => vec![MsgData::Sold(Order {
                        amount: order.amount,
                        quote: order.quote,
                        base: order.base,
                    })],
                }
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

    #[async_std::test]
    async fn should_buy_max_amount_of_quote() {
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
    async fn should_buy_different_max_amount_of_quote() {
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
    async fn should_not_buy_quote_when_no_assets() {
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
    async fn should_not_buy_quote_when_zero() {
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
    async fn should_emit_bought_order_event() {
        let mut exchange = MockExchange::new(Assets {
            quote: Some(Asset {
                amount: 50.0,
                name: "USDT".into(),
            }),
            base: None,
        });
        let mut trader = Trader::new(&mut exchange);

        let actual = trader.act(&Msg::with_data(MsgData::Buy)).await.unwrap();

        let expected = vec![MsgData::Bought(Order {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 50.0,
        })];
        assert_eq!(expected, actual)
    }

    #[async_std::test]
    async fn should_sell_max_amount_of_base() {
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
    async fn should_sell_different_max_amount_of_base() {
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
    async fn should_not_sell_when_no_assets() {
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
    async fn should_not_sell_when_base_zero() {
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
    async fn should_emit_sold_order_event() {
        let mut exchange = MockExchange::new(Assets {
            quote: None,
            base: Some(Asset {
                amount: 0.0000001,
                name: "BTC".into(),
            }),
        });
        let mut trader = Trader::new(&mut exchange);

        let actual = trader.act(&Msg::with_data(MsgData::Sell)).await.unwrap();

        let expected = vec![MsgData::Sold(Order {
            base: "BTC".into(),
            quote: "USDT".into(),
            amount: 0.0000001,
        })];
        assert_eq!(expected, actual)
    }
}
